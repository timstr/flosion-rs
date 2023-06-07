use super::{
    path::SoundPath, soundinput::SoundInputId, soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId, soundprocessor::SoundProcessorId,
};

#[derive(Debug, Eq, PartialEq)]
pub enum SoundError {
    ProcessorIdTaken(SoundProcessorId),
    ProcessorNotFound(SoundProcessorId),
    BadProcessorInit(SoundProcessorId),
    BadProcessorCleanup(SoundProcessorId),
    SoundInputIdTaken(SoundInputId),
    SoundInputNotFound(SoundInputId),
    BadSoundInputInit(SoundInputId),
    BadSoundInputCleanup(SoundInputId),
    BadSoundInputKeyIndex(SoundInputId, usize),
    SoundInputOccupied {
        input_id: SoundInputId,
        current_target: SoundProcessorId,
    },
    SoundInputUnoccupied(SoundInputId),
    CircularDependency {
        cycle: SoundPath,
    },
    StaticTooManyStates(SoundProcessorId),
    StaticNotSynchronous(SoundProcessorId),
    NumberSourceIdTaken(SoundNumberSourceId),
    NumberSourceNotFound(SoundNumberSourceId),
    BadNumberSourceInit(SoundNumberSourceId),
    BadNumberSourceCleanup(SoundNumberSourceId),
    NumberInputIdTaken(SoundNumberInputId),
    BadNumberInputInit(SoundNumberInputId),
    BadNumberInputCleanup(SoundNumberInputId),
    NumberInputNotFound(SoundNumberInputId),
    NumberInputAlreadyConnected {
        input_id: SoundNumberInputId,
        target: SoundNumberSourceId,
    },
    NumberInputNotConnected {
        input_id: SoundNumberInputId,
        target: SoundNumberSourceId,
    },
    StateNotInScope {
        bad_dependencies: Vec<(SoundNumberSourceId, SoundNumberInputId)>,
    },
}
