use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numbersource::{NumberConfig, NumberSourceHandle},
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::DynamicSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
};

pub struct WaveGenerator {
    pub phase: NumberSourceHandle,
    pub amplitude: NumberInputHandle,
    pub frequency: NumberInputHandle,
}

pub struct WaveGeneratorState {
    phase: [f32; CHUNK_SIZE],
}

impl Default for WaveGeneratorState {
    fn default() -> WaveGeneratorState {
        WaveGeneratorState {
            phase: [0.0; CHUNK_SIZE],
        }
    }
}

impl SoundState for WaveGeneratorState {
    fn reset(&mut self) {
        numeric::fill(&mut self.phase, 0.0);
    }
}

impl DynamicSoundProcessor for WaveGenerator {
    type StateType = WaveGeneratorState;

    fn new(tools: &mut SoundProcessorTools<'_, WaveGeneratorState>) -> WaveGenerator {
        WaveGenerator {
            phase: tools
                .add_processor_number_source(|dst: &mut [f32], state: &WaveGeneratorState| {
                    numeric::copy(&state.phase, dst);
                })
                .0,
            amplitude: tools.add_number_input().0,
            frequency: tools.add_number_input().0,
        }
    }

    fn process_audio(
        &self,
        dst: &mut SoundChunk,
        context: ProcessorContext<'_, WaveGeneratorState>,
    ) {
        {
            let mut state = context.write_state();
            let phase_arr = &mut state.phase;
            let prev_phase = *phase_arr.last().unwrap();
            self.frequency.eval(
                phase_arr,
                context.number_context(NumberConfig::samplewise_temporal_at(0)),
            );
            numeric::div_scalar_inplace(phase_arr, SAMPLE_FREQUENCY as f32);
            numeric::exclusive_scan_inplace(phase_arr, prev_phase, |p1, p2| p1 + p2);
            numeric::apply_unary_inplace(phase_arr, |x| x - x.floor());
        }
        self.amplitude.eval(
            &mut dst.l,
            context.number_context(NumberConfig::samplewise_temporal_at(0)),
        );
        numeric::copy(&dst.l, &mut dst.r);
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}
