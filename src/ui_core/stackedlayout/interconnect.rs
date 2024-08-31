use std::hash::Hasher;

use hashrevise::{Revisable, RevisionHash, RevisionHasher};

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

impl Revisable for ProcessorPlug {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revisable(&self.processor);
        hasher.write_u8(self.is_static as _);
        hasher.into_revision()
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

impl Revisable for InputSocket {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_revisable(&self.input);
        hasher.write_u8(match self.options {
            InputOptions::Synchronous => 0,
            InputOptions::NonSynchronous => 1,
        });
        hasher.write_usize(self.branches);
        hasher.into_revision()
    }
}
