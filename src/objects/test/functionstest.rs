use rand::prelude::*;

use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    core::{
        engine::{
            nodegen::NodeGen,
            scratcharena::ScratchArena,
            soundnumberinputnode::{
                SoundNumberInputNode, SoundNumberInputNodeCollection, SoundNumberInputNodeVisitor,
                SoundNumberInputNodeVisitorMut,
            },
        },
        graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
        jit::{codegen::CodeGen, compilednumberinput::Discretization},
        number::{numbergraphdata::NumberTarget, numbersource::PureNumberSource},
        sound::{
            context::{Context, LocalArrayList},
            soundedit::SoundEdit,
            soundgraphdata::{SoundNumberInputScope, SoundProcessorData},
            soundgraphtopology::SoundGraphTopology,
            soundinput::SoundInputId,
            soundnumberinput::{SoundNumberInputHandle, SoundNumberInputId},
            soundnumbersource::{SoundNumberSourceHandle, SoundNumberSourceId},
            soundprocessor::{
                DynamicSoundProcessor, DynamicSoundProcessorWithId, SoundProcessorId,
                StateAndTiming, StreamStatus,
            },
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::SoundChunk,
        uniqueid::IdGenerator,
    },
    objects::purefunctions::*,
};

const TEST_ARRAY_SIZE: usize = 1024;

const MAX_NUM_INPUTS: usize = 3;

struct TestSoundProcessor {
    number_input: SoundNumberInputHandle,
    input_values: Mutex<[[f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS]>,
    number_source_0: SoundNumberSourceHandle,
    number_source_1: SoundNumberSourceHandle,
    number_source_2: SoundNumberSourceHandle,
}

struct TestNumberInput<'ctx> {
    input: SoundNumberInputNode<'ctx>,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for TestNumberInput<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.input);
    }

    fn visit_number_inputs_mut(
        &mut self,
        visitor: &'_ mut dyn SoundNumberInputNodeVisitorMut<'ctx>,
    ) {
        visitor.visit_node(&mut self.input);
    }
}
struct TestSoundProcessorState {
    values: [[f32; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS],
}

impl State for TestSoundProcessorState {
    fn reset(&mut self) {
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

    type NumberInputType<'ctx> = TestNumberInput<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(TestSoundProcessor {
            number_input: tools
                .add_number_input(0.0, SoundNumberInputScope::with_processor_state()),
            input_values: Mutex::new([[0.0; TEST_ARRAY_SIZE]; MAX_NUM_INPUTS]),
            number_source_0: tools.add_processor_array_number_source(|data| {
                &data
                    .downcast_if::<TestSoundProcessorState>()
                    .unwrap()
                    .values[0]
            }),
            number_source_1: tools.add_processor_array_number_source(|data| {
                &data
                    .downcast_if::<TestSoundProcessorState>()
                    .unwrap()
                    .values[1]
            }),
            number_source_2: tools.add_processor_array_number_source(|data| {
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

    fn make_number_inputs<'ctx>(&self, context: &NodeGen<'_, 'ctx>) -> Self::NumberInputType<'ctx> {
        TestNumberInput {
            input: self.number_input.make_node(context),
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
        _number_inputs: &mut Self::NumberInputType<'ctx>,
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

fn do_number_source_test<T: PureNumberSource, F: Fn(&[f32]) -> f32>(
    input_ranges: &[(f32, f32)],
    test_function: F,
) {
    let mut topo = SoundGraphTopology::new();

    let mut spidgen = IdGenerator::<SoundProcessorId>::new();
    let mut siidgen = IdGenerator::<SoundInputId>::new();
    let mut nsidgen = IdGenerator::<SoundNumberSourceId>::new();
    let mut niidgen = IdGenerator::<SoundNumberInputId>::new();

    let test_spid = spidgen.next_id();
    let time_nsid = nsidgen.next_id();

    // for stuff added via number source tools or sound processor tools
    let mut edit_queue = Vec::new();

    // create test sound processor
    let tools = SoundProcessorTools::new(
        test_spid,
        &mut siidgen,
        &mut niidgen,
        &mut nsidgen,
        &mut edit_queue,
    );
    let init = ObjectInitialization::Default;
    let sp_instance = Arc::new(DynamicSoundProcessorWithId::new(
        TestSoundProcessor::new(tools, init).unwrap(),
        test_spid,
        time_nsid,
    ));
    let sp_instance_2 = Arc::clone(&sp_instance);

    // add sound processor to topology
    topo.make_sound_edit(SoundEdit::AddSoundProcessor(SoundProcessorData::new(
        sp_instance_2,
    )));

    // flush other edits to topology
    for edit in edit_queue {
        topo.make_sound_graph_edit(edit);
    }

    {
        let number_input_data = topo
            .number_input_mut(sp_instance.number_input.id())
            .unwrap();

        let (number_graph, mapping) = number_input_data.number_graph_and_mapping_mut();

        let giid0 = mapping.add_target(sp_instance.number_source_0.id(), number_graph);
        let giid1 = mapping.add_target(sp_instance.number_source_1.id(), number_graph);
        let giid2 = mapping.add_target(sp_instance.number_source_2.id(), number_graph);

        let ns_handle = number_graph
            .add_pure_number_source::<T>(ObjectInitialization::Default)
            .unwrap();

        let input_ids = number_graph
            .topology()
            .number_source(ns_handle.id())
            .unwrap()
            .number_inputs()
            .to_vec();

        for (niid, giid) in input_ids.into_iter().zip(
            [giid0, giid1, giid2]
                .into_iter()
                .map(Some)
                .chain(std::iter::repeat(None)),
        ) {
            if let Some(giid) = giid {
                number_graph
                    .connect_number_input(niid, NumberTarget::GraphInput(giid))
                    .unwrap();
            } else {
                panic!("A number source has more than three inputs and not all are being tested");
            }
        }

        number_graph
            .connect_graph_output(
                number_graph.topology().graph_outputs()[0].id(),
                NumberTarget::Source(ns_handle.id()),
            )
            .unwrap();
    }

    //------------------------

    let inkwell_context = inkwell::context::Context::create();

    let codegen = CodeGen::new(&inkwell_context);

    let compiled_input = codegen.compile_number_input(sp_instance.number_input.id(), &topo);

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

fn do_number_source_test_unary<T: PureNumberSource>(
    input_range: (f32, f32),
    test_function: fn(f32) -> f32,
) {
    do_number_source_test::<T, _>(&[input_range], |inputs| test_function(inputs[0]))
}

fn do_number_source_test_binary<T: PureNumberSource>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    test_function: fn(f32, f32) -> f32,
) {
    do_number_source_test::<T, _>(&[input0_range, input1_range], |inputs| {
        test_function(inputs[0], inputs[1])
    })
}

fn do_number_source_test_ternary<T: PureNumberSource>(
    input0_range: (f32, f32),
    input1_range: (f32, f32),
    input2_range: (f32, f32),
    test_function: fn(f32, f32, f32) -> f32,
) {
    do_number_source_test::<T, _>(&[input0_range, input1_range, input2_range], |inputs| {
        test_function(inputs[0], inputs[1], inputs[2])
    })
}

#[test]
fn test_negate() {
    do_number_source_test_unary::<Negate>((-10.0, 10.0), |x| -x);
}

#[test]
fn test_floor() {
    do_number_source_test_unary::<Floor>((-10.0, 10.0), |x| x.floor());
}

#[test]
fn test_ceil() {
    do_number_source_test_unary::<Ceil>((-10.0, 10.0), |x| x.ceil());
}

#[test]
fn test_round() {
    do_number_source_test_unary::<Round>((-10.0, 10.0), |x| x.round());
}

#[test]
fn test_trunc() {
    do_number_source_test_unary::<Trunc>((-10.0, 10.0), |x| x.trunc());
}

#[test]
fn test_fract() {
    do_number_source_test_unary::<Fract>((-10.0, 10.0), |x| x.fract());
}

#[test]
fn test_abs() {
    do_number_source_test_unary::<Abs>((-10.0, 10.0), |x| x.abs());
}

#[test]
fn test_signum() {
    do_number_source_test_unary::<Signum>((-10.0, 10.0), |x| x.signum());
}

#[test]
fn test_exp() {
    do_number_source_test_unary::<Exp>((-10.0, 10.0), |x| x.exp());
}

#[test]
fn test_exp2() {
    do_number_source_test_unary::<Exp2>((-10.0, 10.0), |x| x.exp2());
}

#[test]
fn test_exp10() {
    do_number_source_test_unary::<Exp10>((-10.0, 10.0), |x| (x * std::f32::consts::LN_10).exp());
}

#[test]
fn test_log() {
    do_number_source_test_unary::<Log>((0.0, 10.0), |x| x.ln());
}

#[test]
fn test_log2() {
    do_number_source_test_unary::<Log2>((0.0, 10.0), |x| x.log2());
}

#[test]
fn test_log10() {
    do_number_source_test_unary::<Log10>((0.0, 10.0), |x| x.log10());
}

#[test]
fn test_sqrt() {
    do_number_source_test_unary::<Sqrt>((0.0, 10.0), |x| x.sqrt());
}

#[test]
fn test_sin() {
    do_number_source_test_unary::<Sin>((-10.0, 10.0), |x| x.sin());
}

#[test]
fn test_cos() {
    do_number_source_test_unary::<Cos>((-10.0, 10.0), |x| x.cos());
}

#[test]
fn test_sinewave() {
    do_number_source_test_unary::<SineWave>((-10.0, 10.0), |x| (x * std::f32::consts::TAU).sin());
}

#[test]
fn test_cosinewave() {
    do_number_source_test_unary::<CosineWave>((-10.0, 10.0), |x| (x * std::f32::consts::TAU).cos());
}

#[test]
fn test_squarewave() {
    do_number_source_test_unary::<SquareWave>((-10.0, 10.0), |x| {
        if (x - x.floor()) >= 0.5 {
            1.0
        } else {
            -1.0
        }
    });
}

#[test]
fn test_sawwave() {
    do_number_source_test_unary::<SawWave>((-10.0, 10.0), |x| 2.0 * (x - x.floor()) - 1.0);
}

#[test]
fn test_trianglewave() {
    do_number_source_test_unary::<TriangleWave>((-10.0, 10.0), |x| {
        4.0 * (x - (x + 0.5).floor()).abs() - 1.0
    });
}

#[test]
fn test_add() {
    do_number_source_test_binary::<Add>((-10.0, 10.0), (-10.0, 10.0), |a, b| a + b);
}

#[test]
fn test_subtract() {
    do_number_source_test_binary::<Subtract>((-10.0, 10.0), (-10.0, 10.0), |a, b| a - b);
}

#[test]
fn test_multiply() {
    do_number_source_test_binary::<Multiply>((-10.0, 10.0), (-10.0, 10.0), |a, b| a * b);
}

#[test]
fn test_divide() {
    do_number_source_test_binary::<Divide>((-10.0, 10.0), (-10.0, 10.0), |a, b| a / b);
}

#[test]
fn test_copysign() {
    do_number_source_test_binary::<Copysign>((-10.0, 10.0), (-10.0, 10.0), |a, b| a.copysign(b));
}

#[test]
fn test_pow() {
    do_number_source_test_binary::<Pow>((-10.0, 10.0), (-10.0, 10.0), |a, b| a.powf(b));
}

#[test]
fn test_lerp() {
    do_number_source_test_ternary::<Lerp>(
        (-10.0, 10.0),
        (-10.0, 10.0),
        (-10.0, 10.0),
        |a, b, c| a + c * (b - a),
    );
}
