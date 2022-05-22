use super::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

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
