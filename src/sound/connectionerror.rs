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
    CircularDependency { cycle: NumberPath },
    InputNotFound(NumberInputId),
    InputOccupied(NumberInputId, NumberSourceId),
    SourceNotFound(NumberSourceId),
    StateNotInScope { path: NumberPath },
}

#[derive(Debug)]
pub enum ConnectionError {
    Number(NumberConnectionError),
    Sound(SoundConnectionError),
}

impl From<SoundConnectionError> for ConnectionError {
    fn from(e: SoundConnectionError) -> ConnectionError {
        ConnectionError::Sound(e)
    }
}

impl From<NumberConnectionError> for ConnectionError {
    fn from(e: NumberConnectionError) -> ConnectionError {
        ConnectionError::Number(e)
    }
}
