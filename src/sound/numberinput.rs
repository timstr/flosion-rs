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

#[derive(Debug)]
pub enum NumberInputOwner {
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
    NumberSource(NumberSourceId),
}

impl NumberInputOwner {
    pub fn is_stateful(&self) -> bool {
        match self {
            NumberInputOwner::SoundProcessor(_) => true,
            NumberInputOwner::SoundInput(_) => true,
            NumberInputOwner::NumberSource(_) => false,
        }
    }
}
