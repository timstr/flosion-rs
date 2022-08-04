use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numbersource::StateNumberSourceHandle,
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::SoundProcessor,
    soundprocessortools::SoundProcessorTools,
    statetree::{NoInputs, NumberInputNode, State},
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

impl SoundProcessor for WaveGenerator {
    const IS_STATIC: bool = false;

    type StateType = WaveGeneratorState;

    type InputType = NoInputs;

    fn new(mut tools: SoundProcessorTools) -> Self {
        WaveGenerator {
            phase: tools.add_processor_number_source::<Self, _>(
                |dst: &mut [f32], state: &WaveGeneratorState| {
                    numeric::copy(&state.phase, dst);
                },
            ),
            time: tools.add_processor_time(),
            amplitude: tools.add_number_input(),
            frequency: tools.add_number_input(),
            input: NoInputs::default(),
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        todo!()
    }

    fn process_audio(
        state: &mut WaveGeneratorState,
        _inputs: &mut NoInputs,
        dst: &mut SoundChunk,
        context: Context,
    ) {
        let phase_arr = &mut state.phase;
        let prev_phase = *phase_arr.last().unwrap();
        // TODO: mark phase_arr as samplewise temporal
        state.frequency.eval(phase_arr, &context);
        numeric::div_scalar_inplace(phase_arr, SAMPLE_FREQUENCY as f32);
        numeric::exclusive_scan_inplace(phase_arr, prev_phase, |p1, p2| p1 + p2);
        numeric::apply_unary_inplace(phase_arr, |x| x - x.floor());
        // TODO: mark dst.l as samplewise temporal
        state.amplitude.eval(&mut dst.l, &context);
        numeric::copy(&dst.l, &mut dst.r);
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}
