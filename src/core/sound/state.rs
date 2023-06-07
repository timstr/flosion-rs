use super::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

pub trait State: Sync + Send + 'static {
    fn reset(&mut self);
}

impl State for () {
    fn reset(&mut self) {}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum StateOwner {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
}
