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
