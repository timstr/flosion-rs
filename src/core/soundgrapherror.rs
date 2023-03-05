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
    ProcessorNotFound(SoundProcessorId),
    BadProcessorInit(SoundProcessorId),
    BadProcessorCleanup(SoundProcessorId),
    InputIdTaken(SoundInputId),
    InputNotFound(SoundInputId),
    BadInputInit(SoundInputId),
    BadInputCleanup(SoundInputId),
    BadInputKeyIndex(SoundInputId, usize),
    InputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
    InputUnoccupied(SoundInputId),
    CircularDependency {
        cycle: SoundPath,
    },
    StaticTooManyStates(SoundProcessorId),
    StaticNotSynchronous(SoundProcessorId),
}

#[derive(Debug, Eq, PartialEq)]
pub enum NumberError {
    SourceIdTaken(NumberSourceId),
    SourceNotFound(NumberSourceId),
    BadSourceInit(NumberSourceId),
    BadSourceCleanup(NumberSourceId),
    InputIdTaken(NumberInputId),
    InputNotFound(NumberInputId),
    BadInputInit(NumberInputId),
    BadInputCleanup(NumberInputId),
    InputOccupied {
        input_id: NumberInputId,
        current_target: NumberSourceId,
    },
    InputUnoccupied(NumberInputId),
    CircularDependency {
        cycle: NumberPath,
    },
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
