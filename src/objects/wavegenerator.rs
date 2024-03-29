use crate::core::{
    anydata::AnyData,
    engine::{
        nodegen::NodeGen,
        soundnumberinputnode::{
            SoundNumberInputNode, SoundNumberInputNodeCollection, SoundNumberInputNodeVisitor,
            SoundNumberInputNodeVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compilednumberinput::Discretization,
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::{Context, LocalArrayList},
        soundgraphdata::SoundNumberInputScope,
        soundnumberinput::SoundNumberInputHandle,
        soundnumbersource::SoundNumberSourceHandle,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

pub struct WaveGenerator {
    pub phase: SoundNumberSourceHandle,
    pub amplitude: SoundNumberInputHandle,
    pub frequency: SoundNumberInputHandle,
}

pub struct WaveGeneratorNumberInputs<'ctx> {
    frequency: SoundNumberInputNode<'ctx>,
    amplitude: SoundNumberInputNode<'ctx>,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for WaveGeneratorNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.frequency);
        visitor.visit_node(&self.amplitude);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.frequency);
        visitor.visit_node(&mut self.amplitude);
    }
}

pub struct WaveGeneratorState {
    phase: [f32; CHUNK_SIZE],
}

impl State for WaveGeneratorState {
    fn reset(&mut self) {
        slicemath::fill(&mut self.phase, 0.0);
    }
}

impl DynamicSoundProcessor for WaveGenerator {
    type StateType = WaveGeneratorState;
    type SoundInputType = ();
    type NumberInputType<'ctx> = WaveGeneratorNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WaveGenerator {
            // TODO: bypass this array entirely?
            phase: tools.add_processor_array_number_source(|state: &AnyData| -> &[f32] {
                &state.downcast_if::<WaveGeneratorState>().unwrap().phase
            }),
            amplitude: tools.add_number_input(0.0, SoundNumberInputScope::with_processor_state()),
            frequency: tools.add_number_input(250.0, SoundNumberInputScope::with_processor_state()),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        WaveGeneratorState {
            phase: [0.0; CHUNK_SIZE],
        }
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        WaveGeneratorNumberInputs {
            frequency: self.frequency.make_node(nodegen),
            amplitude: self.amplitude.make_node(nodegen),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<WaveGeneratorState>,
        _sound_inputs: &mut (),
        number_inputs: &mut WaveGeneratorNumberInputs,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let prev_phase = *state.phase.last().unwrap();
        {
            let mut tmp = context.get_scratch_space(state.phase.len());
            number_inputs.frequency.eval(
                &mut tmp,
                Discretization::samplewise_temporal(),
                &context.push_processor_state(state, LocalArrayList::new()),
            );
            slicemath::copy(&tmp, &mut state.phase);
        }
        slicemath::div_scalar_inplace(&mut state.phase, SAMPLE_FREQUENCY as f32);
        slicemath::exclusive_scan_inplace(&mut state.phase, prev_phase, |p1, p2| p1 + p2);
        slicemath::apply_unary_inplace(&mut state.phase, |x| x - x.floor());

        number_inputs.amplitude.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(state, LocalArrayList::new()),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}
