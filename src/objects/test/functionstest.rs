use rand::prelude::*;

use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    core::{
        engine::{
            compiledexpression::{
                CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor,
                CompiledExpressionVisitorMut,
            },
            scratcharena::ScratchArena,
            soundgraphcompiler::SoundGraphCompiler,
        },
        expression::{expressiongraphdata::ExpressionTarget, expressionnode::PureExpressionNode},
        graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
        jit::{codegen::CodeGen, compiledexpression::Discretization},
        sound::{
            context::{Context, LocalArrayList},
            expression::SoundExpressionHandle,
            expressionargument::SoundExpressionArgumentHandle,
            soundgraph::SoundGraphIdGenerators,
            soundgraphdata::{SoundExpressionScope, SoundProcessorData},
            soundgraphtopology::SoundGraphTopology,
            soundprocessor::{
                DynamicSoundProcessor, DynamicSoundProcessorWithId, SoundProcessorId,
                StateAndTiming, StreamStatus,
            },
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::SoundChunk,
    },
    objects::purefunctions::*,
};

const TEST_ARRAY_SIZE: usize = 1024;

const MAX_NUM_INPUTS: usize = 3;

struct TestSoundProcessor {
    expression: SoundExpressionHandle,
    input_values: Mutex<[[f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS]>,
    argument_0: SoundExpressionArgumentHandle,
    argument_1: SoundExpressionArgumentHandle,
    argument_2: SoundExpressionArgumentHandle,
}

struct TestExpressions<'ctx> {
    input: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for TestExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.input);
    }

    fn visit_mut(&mut self, visitor: &'_ mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.input);
    }
}
struct TestSoundProcessorState {
    values: [[f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS],
}

impl State for TestSoundProcessorState {
    fn start_over(&mut self) {
        // NOTE: values shouldn't be overwritten
    }
}
impl TestSoundProcessor {
    fn set_input_values(&self, values: [[f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS]) {
        *self.input_values.lock() = values;
    }
}

impl DynamicSoundProcessor for TestSoundProcessor {
    type StateType = TestSoundProcessorState;

    type SoundInputType = ();

    type Expressions<'ctx> = TestExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(TestSoundProcessor {
            expression: tools.add_expression(0.0, SoundExpressionScope::with_processor_state()),
            input_values: Mutex::new([[0.0; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS]),
            argument_0: tools.add_processor_array_argument(|data| {
                &data
                    .downcast_if::<TestSoundProcessorState>()
                    .unwrap()
                    .values[0]
            }),
            argument_1: tools.add_processor_array_argument(|data| {
                &data
                    .downcast_if::<TestSoundProcessorState>()
                    .unwrap()
                    .values[1]
            }),
            argument_2: tools.add_processor_array_argument(|data| {
                &data
                    .downcast_if::<TestSoundProcessorState>()
                    .unwrap()
                    .values[2]
            }),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn compile_expressions<'ctx>(
        &self,
        context: &SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        TestExpressions {
            input: self.expression.compile(context),
        }
    }

    fn make_state(&self) -> Self::StateType {
        TestSoundProcessorState {
            values: *self.input_values.lock(),
        }
    }

    fn process_audio<'ctx>(
        _state: &mut StateAndTiming<Self::StateType>,
        _sound_inputs: &mut (),
        _expressions: &mut Self::Expressions<'ctx>,
        _dst: &mut SoundChunk,
        _context: Context,
    ) -> StreamStatus {
        panic!("Not used")
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
                diff < 1e-3 * mag,
                "Expected something near {} but instead got {}",
                $expected,
                $actual,
            )
        }
    };
}

fn do_expression_test<T: PureExpressionNode, F: Fn(&[f32]) -> f32>(
    input_ranges: &[(f32, f32)],
    test_function: F,
) {
    let mut topo = SoundGraphTopology::new();

    let mut idgens = SoundGraphIdGenerators::new();

    let test_spid = idgens.sound_processor.next_id();
    let time_nsid = idgens.expression_argument.next_id();

    // Add an empty sound processor first to allow topology changes inside
    // the processor's new() method
    topo.add_sound_processor(SoundProcessorData::new_empty(test_spid))
        .unwrap();

    // create test sound processor
    let tools = SoundProcessorTools::new(test_spid, &mut topo, &mut idgens);
    let init = ObjectInitialization::Default;
    let sp_instance = Arc::new(DynamicSoundProcessorWithId::new(
        TestSoundProcessor::new(tools, init).unwrap(),
        test_spid,
        time_nsid,
    ));
    let sp_instance_2 = Arc::clone(&sp_instance);

    // add the actual sound processor to topology
    topo.sound_processor_mut(test_spid)
        .unwrap()
        .set_processor(sp_instance_2);

    {
        let expression_data = topo.expression_mut(sp_instance.expression.id()).unwrap();

        let (expr_graph, mapping) = expression_data.expression_graph_and_mapping_mut();

        let giid0 = mapping.add_argument(sp_instance.argument_0.id(), expr_graph);
        let giid1 = mapping.add_argument(sp_instance.argument_1.id(), expr_graph);
        let giid2 = mapping.add_argument(sp_instance.argument_2.id(), expr_graph);

        let ns_handle = expr_graph
            .add_pure_expression_node::<T>(ObjectInitialization::Default)
            .unwrap();

        let input_ids = expr_graph
            .topology()
            .node(ns_handle.id())
            .unwrap()
            .inputs()
            .to_vec();

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
                expr_graph.topology().results()[0].id(),
                ExpressionTarget::Node(ns_handle.id()),
            )
            .unwrap();
    }

    //------------------------

    let inkwell_context = inkwell::context::Context::create();

    let codegen = CodeGen::new(&inkwell_context);

    let compiled_input = codegen.compile_expression(sp_instance.expression.id(), &topo);

    let mut compiled_function = compiled_input.make_function();

    let scratch_space = ScratchArena::new();
    let context = Context::new(SoundProcessorId::new(1), &scratch_space);

    //------------------------

    // Fill input arrays with randomly generated values within the desired ranges
    let mut input_values = [[0.0_f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS];
    assert!(input_ranges.len() <= MAX_NUM_INPUTS);
    for (range, values) in input_ranges.into_iter().zip(input_values.iter_mut()) {
        for v in values {
            *v = range.0 + thread_rng().gen::<f32>() * (range.1 - range.0);
        }
    }

    sp_instance.set_input_values(input_values);

    let mut expected_values = [0.0_f32; TEST_ARRAY_SIZE];
    let mut inputs_arr = [0.0_f32; MAX_NUM_INPUTS];
    for i in 0..TEST_ARRAY_SIZE {
        for j in 0..MAX_NUM_INPUTS {
            inputs_arr[j] = input_values[j][i];
        }
        expected_values[i] = test_function(&inputs_arr);
    }
    let expected_values = expected_values;

    //------------------------

    let sp_state = StateAndTiming::new(sp_instance.make_state());
    let context = context.push_processor_state(&sp_state, LocalArrayList::new());

    let state_from_context = context.find_processor_state(sp_instance.id());
    let state_from_context = state_from_context
        .downcast_if::<TestSoundProcessorState>()
        .unwrap();

    for (expected_arr, actual_arr) in input_values
        .into_iter()
        .zip(state_from_context.values.iter().cloned())
    {
        for (expected, actual) in expected_arr.into_iter().zip(actual_arr.iter().cloned()) {
            assert_near!(expected, actual);
        }
    }
    //------------------------

    // test compiled evaluation
    let mut actual_values_compiled = [0.0_f32; TEST_ARRAY_SIZE];
    compiled_function.eval(&mut actual_values_compiled, &context, Discretization::None);

    for (expected, actual) in expected_values
        .into_iter()
        .zip(actual_values_compiled.into_iter())
    {
        assert_near!(expected, actual);
    }
}

fn do_expression_test_unary<T: PureExpressionNode>(
    input_range: (f32, f32),
    test_function: fn(f32) -> f32,
) {
    do_expression_test::<T, _>(&[input_range], |inputs| test_function(inputs[0]))
}

fn do_expression_test_binary<T: PureExpressionNode>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    test_function: fn(f32, f32) -> f32,
) {
    do_expression_test::<T, _>(&[input0_range, input1_range], |inputs| {
        test_function(inputs[0], inputs[1])
    })
}

fn do_expression_test_ternary<T: PureExpressionNode>(
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
