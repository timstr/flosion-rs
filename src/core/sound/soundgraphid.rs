use super::{
    expression::ProcessorExpressionLocation,
    expressionargument::{ArgumentLocation, ProcessorArgumentLocation, SoundInputArgumentLocation},
    soundinput::SoundInputLocation,
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
pub enum SoundGraphComponentLocation {
    Processor(SoundProcessorId),
    Input(SoundInputLocation),
    Expression(ProcessorExpressionLocation),
    ProcessorArgument(ProcessorArgumentLocation),
    InputArgument(SoundInputArgumentLocation),
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
impl From<SoundInputArgumentLocation> for SoundGraphComponentLocation {
    fn from(x: SoundInputArgumentLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::InputArgument(x)
    }
}
impl From<&SoundInputArgumentLocation> for SoundGraphComponentLocation {
    fn from(x: &SoundInputArgumentLocation) -> SoundGraphComponentLocation {
        SoundGraphComponentLocation::InputArgument(*x)
    }
}
impl From<ArgumentLocation> for SoundGraphComponentLocation {
    fn from(x: ArgumentLocation) -> Self {
        match x {
            ArgumentLocation::Processor(x) => SoundGraphComponentLocation::ProcessorArgument(x),
            ArgumentLocation::Input(x) => SoundGraphComponentLocation::InputArgument(x),
        }
    }
}
impl From<&ArgumentLocation> for SoundGraphComponentLocation {
    fn from(x: &ArgumentLocation) -> Self {
        match *x {
            ArgumentLocation::Processor(x) => SoundGraphComponentLocation::ProcessorArgument(x),
            ArgumentLocation::Input(x) => SoundGraphComponentLocation::InputArgument(x),
        }
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
