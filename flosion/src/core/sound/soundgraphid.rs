use hashstash::{Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use super::{
    argument::ProcessorArgumentLocation, expression::ProcessorExpressionLocation,
    soundinput::SoundInputLocation, soundprocessor::SoundProcessorId,
};

#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub enum SoundObjectId {
    Sound(SoundProcessorId),
}

impl Stashable for SoundObjectId {
    fn stash(&self, stasher: &mut Stasher) {
        match self {
            SoundObjectId::Sound(spid) => spid.stash(stasher),
        }
    }
}

impl Unstashable for SoundObjectId {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(SoundObjectId::Sound(SoundProcessorId::unstash(unstasher)?))
    }
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
pub enum SoundGraphComponentLocation {
    Processor(SoundProcessorId),
    Input(SoundInputLocation),
    Expression(ProcessorExpressionLocation),
    ProcessorArgument(ProcessorArgumentLocation),
}

impl From<SoundProcessorId> for SoundGraphComponentLocation {
    fn from(x: SoundProcessorId) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::Processor(x)
    }
}
impl From<&SoundProcessorId> for SoundGraphComponentLocation {
    fn from(x: &SoundProcessorId) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::Processor(*x)
    }
}
impl From<SoundInputLocation> for SoundGraphComponentLocation {
    fn from(x: SoundInputLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::Input(x)
    }
}
impl From<&SoundInputLocation> for SoundGraphComponentLocation {
    fn from(x: &SoundInputLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::Input(*x)
    }
}
impl From<ProcessorExpressionLocation> for SoundGraphComponentLocation {
    fn from(x: ProcessorExpressionLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::Expression(x)
    }
}
impl From<&ProcessorExpressionLocation> for SoundGraphComponentLocation {
    fn from(x: &ProcessorExpressionLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::Expression(*x)
    }
}
impl From<ProcessorArgumentLocation> for SoundGraphComponentLocation {
    fn from(x: ProcessorArgumentLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::ProcessorArgument(x)
    }
}
impl From<&ProcessorArgumentLocation> for SoundGraphComponentLocation {
    fn from(x: &ProcessorArgumentLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::ProcessorArgument(*x)
    }
}

impl From<SoundObjectId> for SoundGraphComponentLocation {
    fn from(id: SoundObjectId) -> SoundGraphComponentLocation {
        match id {
            SoundObjectId::Sound(i) => SoundGraphComponentLocation::Processor(i),
        }
    }
}
impl From<&SoundObjectId> for SoundGraphComponentLocation {
    fn from(id: &SoundObjectId) -> SoundGraphComponentLocation {
        match id {
            SoundObjectId::Sound(i) => SoundGraphComponentLocation::Processor(*i),
        }
    }
}
