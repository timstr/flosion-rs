use super::{numberinput::NumberInputId, numbersource::NumberSourceId, path::NumberPath};

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
}
