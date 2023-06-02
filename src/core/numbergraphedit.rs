use super::{
    numbergraphdata::{NumberInputData, NumberSourceData},
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::NumberSourceId,
};

pub(crate) enum NumberGraphEdit {
    AddNumberInput(NumberInputData),
    RemoveNumberInput(NumberInputId, NumberInputOwner),
    AddNumberSource(NumberSourceData),
    RemoveNumberSource(NumberSourceId),
}
