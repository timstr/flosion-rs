use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use inkwell::values::FloatValue;
use rand::prelude::*;

use crate::{
    core::{
        engine::{scratcharena::ScratchArena, soundgraphcompiler::SoundGraphCompiler},
        expression::{
            context::ExpressionContext,
            expressiongraph::ExpressionTarget,
            expressiongraphvalidation::find_expression_error,
            expressioninput::ExpressionInput,
            expressionnode::{
                AnyExpressionNode, ExpressionNodeVisitor, ExpressionNodeVisitorMut,
                ExpressionNodeWithId, PureExpressionNode,
            },
        },
        jit::{
            argumentstack::ArgumentStack, cache::JitCache, compiledexpression::Discretization,
            jit::Jit,
        },
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::{ArgumentScope, ProcessorArgument, ProcessorArgumentLocation},
            argumenttypes::plainf32array::PlainF32ArrayArgument,
            context::{AudioStack, AudioContext},
            expression::{ExpressionParameterTarget, ProcessorExpression},
            soundgraph::SoundGraph,
            soundprocessor::{
                ProcessorComponent, ProcessorTiming, SoundProcessor, SoundProcessorWithId,
                StreamStatus,
            },
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    objects::purefunctions::*,
    ui_core::arguments::ParsedArguments,
};

// const TEST_ARRAY_SIZE: usize = 1024;
const TEST_ARRAY_SIZE: usize = 10;

const MAX_NUM_INPUTS: usize = 3;

#[derive(ProcessorComponents)]
struct TestSoundProcessor {
    expression: ProcessorExpression,
    argument_0: ProcessorArgument<PlainF32ArrayArgument>,
    argument_1: ProcessorArgument<PlainF32ArrayArgument>,
    argument_2: ProcessorArgument<PlainF32ArrayArgument>,
}

impl SoundProcessor for TestSoundProcessor {
    fn new(_args: &ParsedArguments) -> TestSoundProcessor {
        let argument_0 = ProcessorArgument::new();
        let argument_1 = ProcessorArgument::new();
        let argument_2 = ProcessorArgument::new();
        TestSoundProcessor {
            expression: ProcessorExpression::new(
                0.0,
                ArgumentScope::new(vec![argument_0.id(), argument_1.id(), argument_2.id()]),
            ),
            argument_0,
            argument_1,
            argument_2,
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        _processor: &mut Self::CompiledType<'_>,
        _dst: &mut SoundChunk,
        _context: &mut AudioContext,
    ) -> StreamStatus {
        panic!("unused")
    }
}

impl WithObjectType for TestSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testsoundprocessor");
}

impl Stashable<StashingContext> for TestSoundProcessor {
    fn stash(&self, _stasher: &mut Stasher<StashingContext>) {
        panic!("Unused")
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for TestSoundProcessor {
    fn unstash_inplace(
        &mut self,
        _unstasher: &mut InplaceUnstasher<UnstashingContext<'a>>,
    ) -> Result<(), UnstashError> {
        panic!("unused")
    }
}

struct Identity {
    input: ExpressionInput,
}

impl PureExpressionNode for Identity {
    fn new(_args: &ParsedArguments) -> Self {
        Identity {
            input: ExpressionInput::new(0.123),
        }
    }

    fn compile<'ctx>(&self, _jit: &mut Jit<'ctx>, inputs: &[FloatValue<'ctx>]) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        inputs[0]
    }

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input);
    }

    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input);
    }
}

impl WithObjectType for Identity {
    const TYPE: ObjectType = ObjectType::new("identity");
}

impl Stashable<StashingContext> for Identity {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
    }
}

impl UnstashableInplace for Identity {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)
    }
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

fn do_expression_test<T, F>(input_ranges: &[(f32, f32)], test_function: F)
where
    T: 'static + PureExpressionNode + Stashable<StashingContext> + UnstashableInplace,
    F: Fn(&[f32]) -> f32,
{
    let mut proc = SoundProcessorWithId::<TestSoundProcessor>::new_default();

    let proc_id = proc.id();

    let arg0_id = proc.argument_0.id();
    let arg1_id = proc.argument_1.id();
    let arg2_id = proc.argument_2.id();

    let param0_id = proc
        .expression
        .add_target(ExpressionParameterTarget::Argument(
            ProcessorArgumentLocation::new(proc_id, arg0_id),
        ));
    let param1_id = proc
        .expression
        .add_target(ExpressionParameterTarget::Argument(
            ProcessorArgumentLocation::new(proc_id, arg1_id),
        ));
    let param2_id = proc
        .expression
        .add_target(ExpressionParameterTarget::Argument(
            ProcessorArgumentLocation::new(proc_id, arg2_id),
        ));

    let expr_graph = proc.expression.graph_mut();

    let node = ExpressionNodeWithId::<T>::new_default();
    let node_id = node.id();
    let input_locations = (&node as &dyn AnyExpressionNode).input_locations();

    if input_locations.len() > 3 {
        panic!("An expression node has more than three inputs and not all are being tested");
    }

    expr_graph.add_expression_node(Box::new(node));

    for (input_loc, param_id) in input_locations
        .into_iter()
        .zip([param0_id, param1_id, param2_id])
    {
        expr_graph
            .connect_input(input_loc, Some(ExpressionTarget::Parameter(param_id)))
            .unwrap();
    }

    expr_graph
        .connect_result(
            expr_graph.results()[0].id(),
            ExpressionTarget::Node(node_id),
        )
        .unwrap();

    assert_eq!(find_expression_error(&expr_graph), None);

    //------------------------

    let inkwell_context = inkwell::context::Context::create();

    let mut jit_cache = JitCache::new(&inkwell_context);

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc));

    jit_cache.refresh(&graph);

    let mut compiler = SoundGraphCompiler::new(&graph, &jit_cache);

    // get non-mut reference to processor to allow using other parts of soundgraph
    let proc = graph
        .sound_processor(proc_id)
        .unwrap()
        .downcast::<TestSoundProcessor>()
        .unwrap();

    let mut compiled_proc = proc.compile(proc.id(), &mut compiler);

    let scratch_arena = ScratchArena::new();
    let argument_stack = ArgumentStack::new();
    let stack = AudioStack::Root;
    let processor_timing = ProcessorTiming::new();
    let mut context = AudioContext::new(
        proc_id,
        &processor_timing,
        &scratch_arena,
        argument_stack.view_at_bottom(),
        stack,
    );

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

    compiled_proc.expression.eval(
        &mut actual_values_compiled,
        Discretization::None,
        ExpressionContext::new(&mut context)
            .push(compiled_proc.argument_0, &input_values[0])
            .push(compiled_proc.argument_1, &input_values[1])
            .push(compiled_proc.argument_2, &input_values[2]),
    );

    for (expected, actual) in expected_values
        .into_iter()
        .zip(actual_values_compiled.into_iter())
    {
        assert_near!(expected, actual);
    }
}

fn do_expression_test_unary<T>(input_range: (f32, f32), test_function: fn(f32) -> f32)
where
    T: 'static + PureExpressionNode + Stashable<StashingContext> + UnstashableInplace,
{
    do_expression_test::<T, _>(&[input_range], |inputs| test_function(inputs[0]))
}

fn do_expression_test_binary<T>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    test_function: fn(f32, f32) -> f32,
) where
    T: 'static + PureExpressionNode + Stashable<StashingContext> + UnstashableInplace,
{
    do_expression_test::<T, _>(&[input0_range, input1_range], |inputs| {
        test_function(inputs[0], inputs[1])
    })
}

fn do_expression_test_ternary<T>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    input2_range: (f32, f32),
    test_function: fn(f32, f32, f32) -> f32,
) where
    T: 'static + PureExpressionNode + Stashable<StashingContext> + UnstashableInplace,
{
    do_expression_test::<T, _>(&[input0_range, input1_range, input2_range], |inputs| {
        test_function(inputs[0], inputs[1], inputs[2])
    })
}

#[test]
fn test_identity() {
    do_expression_test_unary::<Identity>((-10.0, 10.0), |x| x);
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
