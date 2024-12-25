use hashstash::{Stashable, Stasher};

use crate::core::{
    sound::{
        soundinput::{BasicProcessorInput, Chronicity, SoundInputLocation},
        soundprocessor::{AnySoundProcessor, SoundProcessorId},
    },
    stashing::StashingContext,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessorPlug {
    pub(crate) processor: SoundProcessorId,
    pub(crate) is_static: bool,
}

impl ProcessorPlug {
    pub(crate) fn from_processor_data(data: &dyn AnySoundProcessor) -> ProcessorPlug {
        ProcessorPlug {
            processor: data.id(),
            is_static: data.is_static(),
        }
    }
}

impl Stashable<StashingContext> for ProcessorPlug {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.processor.value() as _);
        stasher.bool(self.is_static);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct InputSocket {
    pub(crate) location: SoundInputLocation,
    pub(crate) chronicity: Chronicity,
    pub(crate) branches: usize,
}

impl InputSocket {
    pub(crate) fn from_input_data(
        processor_id: SoundProcessorId,
        data: &BasicProcessorInput,
    ) -> InputSocket {
        InputSocket {
            location: SoundInputLocation::new(processor_id, data.id()),
            chronicity: data.chronicity(),
            branches: data.branches(),
        }
    }
}

impl Stashable<StashingContext> for InputSocket {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.location.processor().value() as _);
        stasher.u64(self.location.input().value() as _);
        stasher.u8(match self.chronicity {
            Chronicity::Iso => 0,
            Chronicity::Aniso => 1,
        });
        stasher.u64(self.branches as _);
    }
}
