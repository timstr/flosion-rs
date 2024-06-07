use super::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

pub trait State: Sync + Send + 'static {
    fn start_over(&mut self);
}

impl State for () {
    fn start_over(&mut self) {}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum StateOwner {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
}
