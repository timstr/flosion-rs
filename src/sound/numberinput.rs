use super::{
    numbersource::NumberSourceId, soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberInputId(usize);

impl Default for NumberInputId {
    fn default() -> NumberInputId {
        NumberInputId(1)
    }
}

impl UniqueId for NumberInputId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> NumberInputId {
        NumberInputId(self.0 + 1)
    }
}

pub enum NumberInputOwner {
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
    NumberSource(NumberSourceId),
}
