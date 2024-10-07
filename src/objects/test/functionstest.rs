use rand::prelude::*;

use crate::{
    core::{
        engine::{
            compiledexpression::CompiledExpression, scratcharena::ScratchArena,
            soundgraphcompiler::SoundGraphCompiler,
        },
        expression::{
            context::ExpressionContext, expressiongraphdata::ExpressionTarget,
            expressionnode::PureExpressionNode,
        },
        jit::{cache::JitCache, compiledexpression::Discretization},
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList, Stack},
            expression::{ProcessorExpression, ProcessorExpressionLocation, SoundExpressionScope},
            expressionargument::{ArgumentLocation, ProcessorArgument, ProcessorArgumentLocation},
            soundgraph::SoundGraph,
            soundprocessor::{
                ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                ProcessorTiming, SoundProcessorId, StreamStatus, WhateverCompiledSoundProcessor,
                WhateverSoundProcessor,
            },
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    objects::purefunctions::*,
    ui_core::arguments::ParsedArguments,
};

const TEST_ARRAY_SIZE: usize = 1024;

const MAX_NUM_INPUTS: usize = 3;

struct TestSoundProcessor {
    expression: ProcessorExpression,
    input_values: [[f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS],
    argument_0: ProcessorArgument,
    argument_1: ProcessorArgument,
    argument_2: ProcessorArgument,
}

struct CompiledTestSoundProcessor<'ctx> {
    expression: CompiledExpression<'ctx>,
}

impl WhateverSoundProcessor for TestSoundProcessor {
    type CompiledType<'ctx> = CompiledTestSoundProcessor<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> TestSoundProcessor {
        TestSoundProcessor {
            expression: tools.make_expression(0.0, SoundExpressionScope::with_processor_state()),
            input_values: [[0.0; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS],
            argument_0: tools.make_local_array_argument(),
            argument_1: tools.make_local_array_argument(),
            argument_2: tools.make_local_array_argument(),
        }
    }

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        self.expression.visit(visitor);
        self.argument_0.visit(visitor);
        self.argument_1.visit(visitor);
        self.argument_2.visit(visitor);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        self.expression.visit_mut(visitor);
        self.argument_0.visit_mut(visitor);
        self.argument_1.visit_mut(visitor);
        self.argument_2.visit_mut(visitor);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn compile<'ctx>(
        &self,
        id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledTestSoundProcessor<'ctx> {
        CompiledTestSoundProcessor {
            expression: self.expression.compile(id, compiler),
        }
    }
}

impl<'ctx> WhateverCompiledSoundProcessor<'ctx> for CompiledTestSoundProcessor<'ctx> {
    fn process_audio(&mut self, _dst: &mut SoundChunk, _context: Context) -> StreamStatus {
        panic!("Unused")
    }

    fn start_over(&mut self) {
        self.expression.start_over();
    }
}

impl WithObjectType for TestSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testsoundprocessor");
}

macro_rules! assert_near {
    ($expected: expr, $actual: expr) => {
        if ($expected).is_nan() {
            assert!(
                ($actual).is_nan(),
                "Expected NaN, but instead got {}",
                $actual
            );
        } else {
            let diff = (($expected) - ($actual)).abs();
            let mag = ($expected).abs().max(($actual).abs()).max(1e-3);
            assert!(
                diff < 1e-3 * mag, // TODO: 1e-3 is a bit permissive is it not?
                "Expected something near {} but instead got {}",
                $expected,
                $actual,
            )
        }
    };
}

fn do_expression_test<T: 'static + PureExpressionNode, F: Fn(&[f32]) -> f32>(
    input_ranges: &[(f32, f32)],
    test_function: F,
) {
    let mut graph = SoundGraph::new();

    let proc = graph
        .add_sound_processor::<TestSoundProcessor>(&ParsedArguments::new_empty())
        .unwrap();

    let proc_id = proc.id();

    {
        let mut proc = proc.get_mut();

        let arg0_id = proc.argument_0.id();
        let arg1_id = proc.argument_1.id();
        let arg2_id = proc.argument_2.id();

        let giid0 = proc.expression.add_argument(ArgumentLocation::Processor(
            ProcessorArgumentLocation::new(proc_id, arg0_id),
        ));
        let giid1 = proc.expression.add_argument(ArgumentLocation::Processor(
            ProcessorArgumentLocation::new(proc_id, arg1_id),
        ));
        let giid2 = proc.expression.add_argument(ArgumentLocation::Processor(
            ProcessorArgumentLocation::new(proc_id, arg2_id),
        ));

        let expr_graph = proc.expression.graph_mut();

        let ns_handle = expr_graph
            .add_pure_expression_node::<T>(&ParsedArguments::new_empty())
            .unwrap();

        let input_ids = expr_graph.node(ns_handle.id()).unwrap().inputs().to_vec();

        for (niid, giid) in input_ids.into_iter().zip(
            [giid0, giid1, giid2]
                .into_iter()
                .map(Some)
                .chain(std::iter::repeat(None)),
        ) {
            if let Some(giid) = giid {
                expr_graph
                    .connect_node_input(niid, ExpressionTarget::Parameter(giid))
                    .unwrap();
            } else {
                panic!(
                    "An expression node has more than three inputs and not all are being tested"
                );
            }
        }

        expr_graph
            .connect_result(
                expr_graph.results()[0].id(),
                ExpressionTarget::Node(ns_handle.id()),
            )
            .unwrap();
    }

    //------------------------

    let inkwell_context = inkwell::context::Context::create();

    let jit_cache = JitCache::new(&inkwell_context);

    let mut compiled_expression;

    {
        let proc = proc.get();

        let location = ProcessorExpressionLocation::new(proc_id, proc.expression.id());

        compiled_expression = jit_cache.get_compiled_expression(
            location,
            proc.expression.graph(),
            proc.expression.mapping(),
            &graph,
        );
    }

    let scratch_arena = ScratchArena::new();
    let stack = Stack::Root;
    let processor_timing = ProcessorTiming::new();
    let context = Context::new(proc_id, &processor_timing, &scratch_arena, stack);

    //------------------------

    // Fill input arrays with randomly generated values within the desired ranges
    let mut input_values = [[0.0_f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS];
    assert!(input_ranges.len() <= MAX_NUM_INPUTS);
    for (range, values) in input_ranges.into_iter().zip(input_values.iter_mut()) {
        for v in values {
            *v = range.0 + thread_rng().gen::<f32>() * (range.1 - range.0);
        }
    }

    let mut expected_values = [0.0_f32; TEST_ARRAY_SIZE];
    {
        let mut inputs_arr = [0.0_f32; MAX_NUM_INPUTS];
        for i in 0..TEST_ARRAY_SIZE {
            for j in 0..MAX_NUM_INPUTS {
                inputs_arr[j] = input_values[j][i];
            }
            expected_values[i] = test_function(&inputs_arr);
        }
    }
    let expected_values = expected_values;

    //------------------------

    // test compiled evaluation
    let mut actual_values_compiled = [0.0_f32; TEST_ARRAY_SIZE];

    let proc = proc.get();
    let arg0_id = proc.argument_0.id();
    let arg1_id = proc.argument_1.id();
    let arg2_id = proc.argument_2.id();

    compiled_expression.eval(
        &mut actual_values_compiled,
        ExpressionContext::new_with_arrays(
            context,
            LocalArrayList::new()
                .push(&input_values[0], arg0_id)
                .push(&input_values[1], arg1_id)
                .push(&input_values[2], arg2_id),
        ),
        Discretization::None,
    );

    for (expected, actual) in expected_values
        .into_iter()
        .zip(actual_values_compiled.into_iter())
    {
        assert_near!(expected, actual);
    }
}

fn do_expression_test_unary<T: 'static + PureExpressionNode>(
    input_range: (f32, f32),
    test_function: fn(f32) -> f32,
) {
    do_expression_test::<T, _>(&[input_range], |inputs| test_function(inputs[0]))
}

fn do_expression_test_binary<T: 'static + PureExpressionNode>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    test_function: fn(f32, f32) -> f32,
) {
    do_expression_test::<T, _>(&[input0_range, input1_range], |inputs| {
        test_function(inputs[0], inputs[1])
    })
}

fn do_expression_test_ternary<T: 'static + PureExpressionNode>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    input2_range: (f32, f32),
    test_function: fn(f32, f32, f32) -> f32,
) {
    do_expression_test::<T, _>(&[input0_range, input1_range, input2_range], |inputs| {
        test_function(inputs[0], inputs[1], inputs[2])
    })
}

#[test]
fn test_negate() {
    do_expression_test_unary::<Negate>((-10.0, 10.0), |x| -x);
}

#[test]
fn test_floor() {
    do_expression_test_unary::<Floor>((-10.0, 10.0), |x| x.floor());
}

#[test]
fn test_ceil() {
    do_expression_test_unary::<Ceil>((-10.0, 10.0), |x| x.ceil());
}

#[test]
fn test_round() {
    do_expression_test_unary::<Round>((-10.0, 10.0), |x| x.round());
}

#[test]
fn test_trunc() {
    do_expression_test_unary::<Trunc>((-10.0, 10.0), |x| x.trunc());
}

#[test]
fn test_fract() {
    do_expression_test_unary::<Fract>((-10.0, 10.0), |x| x.fract());
}

#[test]
fn test_abs() {
    do_expression_test_unary::<Abs>((-10.0, 10.0), |x| x.abs());
}

#[test]
fn test_signum() {
    do_expression_test_unary::<Signum>((-10.0, 10.0), |x| x.signum());
}

#[test]
fn test_exp() {
    do_expression_test_unary::<Exp>((-10.0, 10.0), |x| x.exp());
}

#[test]
fn test_exp2() {
    do_expression_test_unary::<Exp2>((-10.0, 10.0), |x| x.exp2());
}

#[test]
fn test_exp10() {
    do_expression_test_unary::<Exp10>((-10.0, 10.0), |x| (x * std::f32::consts::LN_10).exp());
}

#[test]
fn test_log() {
    do_expression_test_unary::<Log>((0.0, 10.0), |x| x.ln());
}

#[test]
fn test_log2() {
    do_expression_test_unary::<Log2>((0.0, 10.0), |x| x.log2());
}

#[test]
fn test_log10() {
    do_expression_test_unary::<Log10>((0.0, 10.0), |x| x.log10());
}

#[test]
fn test_sqrt() {
    do_expression_test_unary::<Sqrt>((0.0, 10.0), |x| x.sqrt());
}

#[test]
fn test_sin() {
    do_expression_test_unary::<Sin>((-10.0, 10.0), |x| x.sin());
}

#[test]
fn test_cos() {
    do_expression_test_unary::<Cos>((-10.0, 10.0), |x| x.cos());
}

#[test]
fn test_sinewave() {
    do_expression_test_unary::<SineWave>((-10.0, 10.0), |x| (x * std::f32::consts::TAU).sin());
}

#[test]
fn test_cosinewave() {
    do_expression_test_unary::<CosineWave>((-10.0, 10.0), |x| (x * std::f32::consts::TAU).cos());
}

#[test]
fn test_squarewave() {
    do_expression_test_unary::<SquareWave>((-10.0, 10.0), |x| {
        if (x - x.floor()) >= 0.5 {
            1.0
        } else {
            -1.0
        }
    });
}

#[test]
fn test_sawwave() {
    do_expression_test_unary::<SawWave>((-10.0, 10.0), |x| 2.0 * (x - x.floor()) - 1.0);
}

#[test]
fn test_trianglewave() {
    do_expression_test_unary::<TriangleWave>((-10.0, 10.0), |x| {
        4.0 * (x - (x + 0.5).floor()).abs() - 1.0
    });
}

#[test]
fn test_add() {
    do_expression_test_binary::<Add>((-10.0, 10.0), (-10.0, 10.0), |a, b| a + b);
}

#[test]
fn test_subtract() {
    do_expression_test_binary::<Subtract>((-10.0, 10.0), (-10.0, 10.0), |a, b| a - b);
}

#[test]
fn test_multiply() {
    do_expression_test_binary::<Multiply>((-10.0, 10.0), (-10.0, 10.0), |a, b| a * b);
}

#[test]
fn test_divide() {
    do_expression_test_binary::<Divide>((-10.0, 10.0), (-10.0, 10.0), |a, b| a / b);
}

#[test]
fn test_copysign() {
    do_expression_test_binary::<Copysign>((-10.0, 10.0), (-10.0, 10.0), |a, b| a.copysign(b));
}

#[test]
fn test_pow() {
    do_expression_test_binary::<Pow>((-10.0, 10.0), (-10.0, 10.0), |a, b| a.powf(b));
}

#[test]
fn test_lerp() {
    do_expression_test_ternary::<Lerp>((-10.0, 10.0), (-10.0, 10.0), (-10.0, 10.0), |a, b, c| {
        a + c * (b - a)
    });
}
