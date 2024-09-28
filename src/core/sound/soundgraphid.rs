use super::{
    expressionargument::SoundExpressionArgumentId, soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
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
    ExpressionArgument(SoundExpressionArgumentId),
}

impl SoundGraphId {
    pub fn as_usize(&self) -> usize {
        match self {
            SoundGraphId::SoundInput(id) => id.value(),
            SoundGraphId::SoundProcessor(id) => id.value(),
            SoundGraphId::ExpressionArgument(id) => id.value(),
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
impl From<SoundExpressionArgumentId> for SoundGraphId {
    fn from(id: SoundExpressionArgumentId) -> SoundGraphId {
        SoundGraphId::ExpressionArgument(id)
    }
}
impl From<SoundObjectId> for SoundGraphId {
    fn from(id: SoundObjectId) -> SoundGraphId {
        match id {
            SoundObjectId::Sound(i) => SoundGraphId::SoundProcessor(i),
        }
    }
}
impl From<&SoundInputId> for SoundGraphId {
    fn from(id: &SoundInputId) -> SoundGraphId {
        SoundGraphId::SoundInput(*id)
    }
}
impl From<&SoundProcessorId> for SoundGraphId {
    fn from(id: &SoundProcessorId) -> SoundGraphId {
        SoundGraphId::SoundProcessor(*id)
    }
}
impl From<&SoundExpressionArgumentId> for SoundGraphId {
    fn from(id: &SoundExpressionArgumentId) -> SoundGraphId {
        SoundGraphId::ExpressionArgument(*id)
    }
}
impl From<&SoundObjectId> for SoundGraphId {
    fn from(id: &SoundObjectId) -> SoundGraphId {
        match id {
            SoundObjectId::Sound(i) => SoundGraphId::SoundProcessor(*i),
        }
    }
}
