use super::{
    numbergraphdata::NumberInputData,
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::NumberSourceId,
};

pub(crate) enum NumberGraphEdit {
    AddNumberInput(NumberInputData),
    RemoveNumberInput(NumberInputId, NumberInputOwner),
    AddGraphOutput(NumberInputId),
    RemoveGraphOutput(NumberInputId),
    AddGraphInput(NumberSourceId),
    RemoveGraphInput(NumberSourceId),
}
