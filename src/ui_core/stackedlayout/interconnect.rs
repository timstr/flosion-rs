use hashstash::{Stashable, Stasher};

use crate::core::sound::{
    soundgraphdata::{SoundInputData, SoundProcessorData},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessorPlug {
    pub(crate) processor: SoundProcessorId,
    pub(crate) is_static: bool,
}

impl ProcessorPlug {
    pub(crate) fn from_processor_data(data: &SoundProcessorData) -> ProcessorPlug {
        ProcessorPlug {
            processor: data.id(),
            is_static: data.instance().is_static(),
        }
    }
}

impl Stashable for ProcessorPlug {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.u64(self.processor.value() as _);
        stasher.bool(self.is_static);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct InputSocket {
    pub(crate) input: SoundInputId,
    pub(crate) options: InputOptions,
    pub(crate) branches: usize,
}

impl InputSocket {
    pub(crate) fn from_input_data(data: &SoundInputData) -> InputSocket {
        InputSocket {
            input: data.id(),
            options: data.options(),
            branches: data.branches().len(),
        }
    }
}

impl Stashable for InputSocket {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.u64(self.input.value() as _);
        stasher.u8(match self.options {
            InputOptions::Synchronous => 0,
            InputOptions::NonSynchronous => 1,
        });
        stasher.u64(self.branches as _);
    }
}
