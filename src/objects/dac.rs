use crate::sound::soundgraph::{Context, SoundProcessorTools};
use crate::sound::soundinput::InputOptions;
use crate::sound::soundinput::SingleSoundInput;
use crate::sound::soundprocessor::StaticSoundProcessor;
use crate::sound::soundstate::{SoundState, StateTime};

pub struct DAC {
    input: SingleSoundInput,
    // TODO: stuff for actually playing sound to speakers using CPAL
}

pub struct DACState {
    time: StateTime, // TODO: stuff for actually playing sound to speakers using CPAL
}

impl Default for DACState {
    fn default() -> DACState {
        DACState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for DACState {
    fn reset(&mut self) {}

    fn time(&self) -> &StateTime {
        &self.time
    }

    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}

impl DAC {
    pub fn input(&self) -> &SingleSoundInput {
        &self.input
    }
}

impl StaticSoundProcessor for DAC {
    type StateType = DACState;

    fn new(t: &SoundProcessorTools) -> DAC {
        DAC {
            input: t.add_single_input(InputOptions {
                realtime: true,
                interruptible: false,
            }),
        }
    }

    fn process_audio(&self, _state: &mut DACState, _context: &mut Context) {
        // TODO
        println!("DAC processing audio");
    }

    fn produces_output(&self) -> bool {
        false
    }
}
