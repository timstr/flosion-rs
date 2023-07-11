use crate::core::uniqueid::UniqueId;

use super::{
    soundinput::SoundInputId, soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId, soundprocessor::SoundProcessorId,
};

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub enum SoundObjectId {
    Sound(SoundProcessorId),
}

impl SoundObjectId {
    pub fn as_sound_processor_id(&self) -> Option<SoundProcessorId> {
        match self {
            SoundObjectId::Sound(id) => Some(*id),
        }
    }
}

impl From<SoundProcessorId> for SoundObjectId {
    fn from(id: SoundProcessorId) -> SoundObjectId {
        SoundObjectId::Sound(id)
    }
}
impl From<&SoundProcessorId> for SoundObjectId {
    fn from(id: &SoundProcessorId) -> SoundObjectId {
        SoundObjectId::Sound(*id)
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub enum SoundGraphId {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
    SoundNumberInput(SoundNumberInputId),
    SoundNumberSource(SoundNumberSourceId),
}

impl SoundGraphId {
    pub fn as_usize(&self) -> usize {
        match self {
            SoundGraphId::SoundInput(id) => id.value(),
            SoundGraphId::SoundProcessor(id) => id.value(),
            SoundGraphId::SoundNumberInput(id) => id.value(),
            SoundGraphId::SoundNumberSource(id) => id.value(),
        }
    }
}

impl From<SoundInputId> for SoundGraphId {
    fn from(id: SoundInputId) -> SoundGraphId {
        SoundGraphId::SoundInput(id)
    }
}
impl From<SoundProcessorId> for SoundGraphId {
    fn from(id: SoundProcessorId) -> SoundGraphId {
        SoundGraphId::SoundProcessor(id)
    }
}
impl From<SoundNumberInputId> for SoundGraphId {
    fn from(id: SoundNumberInputId) -> SoundGraphId {
        SoundGraphId::SoundNumberInput(id)
    }
}
impl From<SoundNumberSourceId> for SoundGraphId {
    fn from(id: SoundNumberSourceId) -> SoundGraphId {
        SoundGraphId::SoundNumberSource(id)
    }
}
impl From<SoundObjectId> for SoundGraphId {
    fn from(id: SoundObjectId) -> SoundGraphId {
        match id {
            SoundObjectId::Sound(i) => SoundGraphId::SoundProcessor(i),
        }
    }
}
