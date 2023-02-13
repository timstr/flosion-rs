use crate::core::{
    anydata::AnyData,
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numberinputnode::{
        NumberInputNode, NumberInputNodeCollection, NumberInputNodeVisitor,
        NumberInputNodeVisitorMut,
    },
    numbersource::StateNumberSourceHandle,
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
};

pub struct WaveGenerator {
    pub phase: StateNumberSourceHandle,
    pub time: StateNumberSourceHandle,
    pub amplitude: NumberInputHandle,
    pub frequency: NumberInputHandle,
}

pub struct WaveGeneratorNumberInputs<'ctx> {
    frequency: NumberInputNode<'ctx>,
    amplitude: NumberInputNode<'ctx>,
}

impl<'ctx> NumberInputNodeCollection<'ctx> for WaveGeneratorNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.frequency);
        visitor.visit_node(&self.amplitude);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.frequency);
        visitor.visit_node(&mut self.amplitude);
    }
}

pub struct WaveGeneratorState {
    phase: [f32; CHUNK_SIZE],
}

impl State for WaveGeneratorState {
    fn reset(&mut self) {
        numeric::fill(&mut self.phase, 0.0);
    }
}

impl DynamicSoundProcessor for WaveGenerator {
    type StateType = WaveGeneratorState;
    type SoundInputType = ();
    type NumberInputType<'ctx> = WaveGeneratorNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WaveGenerator {
            phase: tools.add_processor_array_number_source(|state: &AnyData| -> &[f32] {
                &state.downcast_if::<WaveGeneratorState>().unwrap().phase
            }),
            time: tools.add_processor_time(),
            amplitude: tools.add_number_input(0.0),
            frequency: tools.add_number_input(250.0),
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

    fn make_number_inputs<'ctx>(
        &self,
        context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx> {
        WaveGeneratorNumberInputs {
            frequency: self.frequency.make_node(context),
            amplitude: self.amplitude.make_node(context),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<WaveGeneratorState>,
        _sound_inputs: &mut (),
        number_inputs: &WaveGeneratorNumberInputs,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let prev_phase = *state.phase.last().unwrap();
        // TODO: mark phase_arr as samplewise temporal
        {
            let mut tmp = context.get_scratch_space(state.phase.len());
            number_inputs
                .frequency
                .eval(&mut tmp, &context.push_processor_state(state));
            numeric::copy(&tmp, &mut state.phase);
        }
        numeric::div_scalar_inplace(&mut state.phase, SAMPLE_FREQUENCY as f32);
        numeric::exclusive_scan_inplace(&mut state.phase, prev_phase, |p1, p2| p1 + p2);
        numeric::apply_unary_inplace(&mut state.phase, |x| x - x.floor());
        // TODO: mark dst.l as samplewise temporal

        number_inputs
            .amplitude
            .eval(&mut dst.l, &context.push_processor_state(state));
        numeric::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}
