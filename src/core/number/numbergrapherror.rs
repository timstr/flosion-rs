use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numbergraphdata::NumberTarget,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    path::NumberPath,
};

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
        current_target: NumberTarget,
    },
    InputUnoccupied(NumberInputId),
    CircularDependency {
        cycle: NumberPath,
    },
    GraphInputIdTaken(NumberGraphInputId),
    GraphInputNotFound(NumberGraphInputId),
    BadGraphInputCleanup(NumberGraphInputId),
    GraphOutputIdTaken(NumberGraphOutputId),
    GraphOutputNotFound(NumberGraphOutputId),
    BadGraphOutputInit(NumberGraphOutputId),
    BadGraphOutputCleanup(NumberGraphOutputId),
    GraphOutputOccupied {
        output_id: NumberGraphOutputId,
        current_target: NumberTarget,
    },
    GraphOutputUnoccupied(NumberGraphOutputId),
}
