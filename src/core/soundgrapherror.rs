use super::{
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    path::{NumberPath, SoundPath},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

#[derive(Debug, Eq, PartialEq)]
pub enum SoundError {
    ProcessorIdTaken(SoundProcessorId),
    InputIdTaken(SoundInputId),
    CircularDependency {
        cycle: SoundPath,
    },
    StaticTooManyStates(SoundProcessorId),
    StaticNotSynchronous(SoundProcessorId),
    ProcessorNotFound(SoundProcessorId),
    InputNotFound(SoundInputId),
    InputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum NumberError {
    SourceIdTaken(NumberSourceId),
    InputIdTaken(NumberInputId),
    CircularDependency {
        cycle: NumberPath,
    },
    InputNotFound(NumberInputId),
    InputOccupied(NumberInputId, NumberSourceId),
    SourceNotFound(NumberSourceId),
    StateNotInScope {
        bad_dependencies: Vec<(NumberSourceId, NumberInputId)>,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum SoundGraphError {
    Number(NumberError),
    Sound(SoundError),
}

impl SoundGraphError {
    pub fn into_number(self) -> Option<NumberError> {
        match self {
            SoundGraphError::Number(e) => Some(e),
            _ => None,
        }
    }

    pub fn into_sound(self) -> Option<SoundError> {
        match self {
            SoundGraphError::Sound(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SoundError> for SoundGraphError {
    fn from(e: SoundError) -> SoundGraphError {
        SoundGraphError::Sound(e)
    }
}

impl From<NumberError> for SoundGraphError {
    fn from(e: NumberError) -> SoundGraphError {
        SoundGraphError::Number(e)
    }
}
