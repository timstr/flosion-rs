use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numbersource::StateNumberSourceHandle,
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::{DynamicSoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{NoInputs, NumberInputNode, State, StateAndTiming},
};

pub struct WaveGenerator {
    pub phase: StateNumberSourceHandle,
    pub time: StateNumberSourceHandle,
    pub amplitude: NumberInputHandle,
    pub frequency: NumberInputHandle,
    input: NoInputs,
}

pub struct WaveGeneratorState {
    phase: [f32; CHUNK_SIZE],
    frequency: NumberInputNode,
    amplitude: NumberInputNode,
}

impl State for WaveGeneratorState {
    fn reset(&mut self) {
        numeric::fill(&mut self.phase, 0.0);
    }
}

impl DynamicSoundProcessor for WaveGenerator {
    type StateType = WaveGeneratorState;

    type InputType = NoInputs;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WaveGenerator {
            phase: tools.add_dynamic_processor_number_source::<Self, _>(
                |dst: &mut [f32], state: &StateAndTiming<WaveGeneratorState>| {
                    numeric::copy(&state.phase, dst);
                },
            ),
            time: tools.add_processor_time(),
            amplitude: tools.add_number_input(0.0),
            frequency: tools.add_number_input(250.0),
            input: NoInputs::default(),
        })
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        WaveGeneratorState {
            phase: [0.0; CHUNK_SIZE],
            frequency: self.frequency.make_node(),
            amplitude: self.amplitude.make_node(),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<WaveGeneratorState>,
        _inputs: &mut NoInputs,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let prev_phase = *state.phase.last().unwrap();
        // TODO: mark phase_arr as samplewise temporal
        {
            let mut tmp = context.get_scratch_space(state.phase.len());
            state
                .frequency
                .eval(tmp.get_mut(), &context.push_processor_state(state));
            numeric::copy(tmp.get(), &mut state.phase);
        }
        numeric::div_scalar_inplace(&mut state.phase, SAMPLE_FREQUENCY as f32);
        numeric::exclusive_scan_inplace(&mut state.phase, prev_phase, |p1, p2| p1 + p2);
        numeric::apply_unary_inplace(&mut state.phase, |x| x - x.floor());
        // TODO: mark dst.l as samplewise temporal

        state
            .amplitude
            .eval(&mut dst.l, &context.push_processor_state(state));
        numeric::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}
