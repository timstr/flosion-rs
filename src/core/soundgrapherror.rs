use super::{
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    path::{NumberPath, SoundPath},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

#[derive(Debug)]
pub enum SoundConnectionError {
    NoChange,
    CircularDependency {
        cycle: SoundPath,
    },
    StaticTooManyStates(SoundProcessorId),
    StaticNotRealtime(SoundProcessorId),
    ProcessorNotFound(SoundProcessorId),
    InputNotFound(SoundInputId),
    InputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
}

#[derive(Debug)]
pub enum NumberConnectionError {
    NoChange,
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

#[derive(Debug)]
pub enum SoundGraphError {
    Number(NumberConnectionError),
    Sound(SoundConnectionError),
}

impl SoundGraphError {
    pub fn into_number(self) -> Option<NumberConnectionError> {
        match self {
            SoundGraphError::Number(e) => Some(e),
            _ => None,
        }
    }

    pub fn into_sound(self) -> Option<SoundConnectionError> {
        match self {
            SoundGraphError::Sound(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SoundConnectionError> for SoundGraphError {
    fn from(e: SoundConnectionError) -> SoundGraphError {
        SoundGraphError::Sound(e)
    }
}

impl From<NumberConnectionError> for SoundGraphError {
    fn from(e: NumberConnectionError) -> SoundGraphError {
        SoundGraphError::Number(e)
    }
}
