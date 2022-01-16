#[derive(Debug)]
pub enum ConnectionError {
    NoChange,
    CircularDependency,
    StaticTooManyStates,
    StaticNotRealtime,
    ProcessorNotFound,
    InputNotFound,
    InputOccupied,
}

pub enum NumberConnectionError {
    NoChange,
    CircularDependency,
    InputNotFound,
    InputOccupied,
    SourceNotFound,
    SourceOutOfScope,
}
