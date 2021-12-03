use crate::sound::soundgraph::{Context, SoundProcessorTools};
use crate::sound::soundprocessor::DynamicSoundProcessor;
use crate::sound::soundstate::{SoundState, StateTime};
use rand::prelude::*;

pub struct WhiteNoise {}

pub struct WhiteNoiseState {
    time: StateTime,
}

impl Default for WhiteNoiseState {
    fn default() -> WhiteNoiseState {
        WhiteNoiseState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for WhiteNoiseState {
    fn reset(&mut self) {}
    fn time(&self) -> &StateTime {
        &self.time
    }
    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = WhiteNoiseState;

    fn new(_tools: SoundProcessorTools) -> WhiteNoise {
        WhiteNoise {}
    }

    fn process_audio(&self, _state: &mut WhiteNoiseState, context: &mut Context) {
        let b = context.output_buffer();
        for s in b.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        for s in b.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
    }
}
