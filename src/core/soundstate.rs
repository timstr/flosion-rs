use super::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

pub struct StateTime {
    elapsed_samples: usize,
    relative_time_speed: f32,
}

impl StateTime {
    pub fn new() -> StateTime {
        StateTime {
            elapsed_samples: 0,
            relative_time_speed: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.elapsed_samples = 0;
        self.relative_time_speed = 1.0;
    }
}

pub trait SoundState: 'static + Sized + Default + Sync + Send {
    fn reset(&mut self);
}

pub struct EmptyState {}

impl Default for EmptyState {
    fn default() -> EmptyState {
        EmptyState {}
    }
}

impl SoundState for EmptyState {
    fn reset(&mut self) {}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StateOwner {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
}
