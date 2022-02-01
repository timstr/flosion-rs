use rand::prelude::*;

use crate::core::{
    context::ProcessorContext, soundchunk::SoundChunk, soundprocessor::DynamicSoundProcessor,
    soundprocessortools::SoundProcessorTools, soundstate::SoundState,
};

pub struct WhiteNoise {}

pub struct WhiteNoiseState {}

impl Default for WhiteNoiseState {
    fn default() -> WhiteNoiseState {
        WhiteNoiseState {}
    }
}

impl SoundState for WhiteNoiseState {
    fn reset(&mut self) {}
}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = WhiteNoiseState;

    fn new(_tools: &mut SoundProcessorTools<WhiteNoiseState>) -> WhiteNoise {
        WhiteNoise {}
    }

    fn process_audio(
        &self,
        dst: &mut SoundChunk,
        mut _context: ProcessorContext<'_, WhiteNoiseState>,
    ) {
        for s in dst.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        for s in dst.r.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
    }
}
