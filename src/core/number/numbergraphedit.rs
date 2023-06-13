use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numbergraphdata::{NumberGraphOutputData, NumberInputData, NumberSourceData, NumberTarget},
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
};

#[derive(Clone)]
pub(crate) enum NumberGraphEdit {
    AddNumberInput(NumberInputData),
    RemoveNumberInput(NumberInputId),
    AddNumberSource(NumberSourceData),
    RemoveNumberSource(NumberSourceId),
    AddGraphInput(NumberGraphInputId),
    RemoveGraphInput(NumberGraphInputId),
    AddGraphOutput(NumberGraphOutputData),
    RemoveGraphOutput(NumberGraphOutputId),
    ConnectNumberInput(NumberInputId, NumberTarget),
    DisconnectNumberInput(NumberInputId),
    ConnectGraphOutput(NumberGraphOutputId, NumberTarget),
    DisconnectGraphOutput(NumberGraphOutputId),
}
