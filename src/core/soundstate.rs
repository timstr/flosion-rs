use super::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

// TODO: why is Sync needed here? Sound states
// should only be accessed by the audio thread
// in theory, so no sharing between threads is
// needed
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
