use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numbergraphdata::{NumberGraphOutputData, NumberInputData, NumberSourceData, NumberTarget},
    numbergrapherror::NumberError,
    numbergraphtopology::NumberGraphTopology,
    numbergraphvalidation::validate_number_connection,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
};

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

impl NumberGraphEdit {
    pub(crate) fn check_preconditions(
        &self,
        topology: &NumberGraphTopology,
    ) -> Option<NumberError> {
        match self {
            NumberGraphEdit::AddNumberInput(data) => {
                // The id must not be taken
                if topology.number_input(data.id()).is_some() {
                    return Some(NumberError::InputIdTaken(data.id()));
                }

                // the owner must exist
                if topology.number_source(data.owner()).is_none() {
                    return Some(NumberError::SourceNotFound(data.owner()));
                }

                // The target must be empty
                if data.target().is_some() {
                    return Some(NumberError::BadInputInit(data.id()));
                }
            }
            NumberGraphEdit::RemoveNumberInput(niid) => {
                // the input must exist
                let Some(data) = topology.number_input(*niid) else {
                    return Some(NumberError::InputNotFound(*niid));
                };

                // the input must be disconnected
                if data.target().is_some() {
                    return Some(NumberError::BadInputCleanup(*niid));
                }
            }
            NumberGraphEdit::AddNumberSource(data) => {
                // the id must not be taken
                if topology.number_source(data.id()).is_some() {
                    return Some(NumberError::SourceIdTaken(data.id()));
                }

                // the source must have no inputs
                if !data.number_inputs().is_empty() {
                    return Some(NumberError::BadSourceInit(data.id()));
                }
            }
            NumberGraphEdit::RemoveNumberSource(nsid) => {
                // the source must exist
                let Some(data) = topology.number_source(*nsid) else {
                    return Some(NumberError::SourceNotFound(*nsid));
                };

                // the source must have no inputs
                if !data.number_inputs().is_empty() {
                    return Some(NumberError::BadSourceCleanup(*nsid));
                }

                // the source must not be connected to any number inputs
                for ni in topology.number_inputs().values() {
                    if ni.target() == Some(NumberTarget::Source(*nsid)) {
                        return Some(NumberError::BadSourceCleanup(*nsid));
                    }
                }

                // the source must not be connected to any graph outputs
                for go in topology.graph_outputs() {
                    if go.target() == Some(NumberTarget::Source(*nsid)) {
                        return Some(NumberError::BadSourceCleanup(*nsid));
                    }
                }
            }
            NumberGraphEdit::AddGraphInput(ngiid) => {
                // The id must not be taken
                if topology.graph_inputs().contains(ngiid) {
                    return Some(NumberError::GraphInputIdTaken(*ngiid));
                }
            }
            NumberGraphEdit::RemoveGraphInput(ngiid) => {
                // the input must exist
                if !topology.graph_inputs().contains(ngiid) {
                    return Some(NumberError::GraphInputNotFound(*ngiid));
                }

                // the input must not be connected to any number inputs
                for ni in topology.number_inputs().values() {
                    if ni.target() == Some(NumberTarget::GraphInput(*ngiid)) {
                        return Some(NumberError::BadGraphInputCleanup(*ngiid));
                    }
                }

                // the input must not be connected to any graph outputs
                for go in topology.graph_outputs() {
                    if go.target() == Some(NumberTarget::GraphInput(*ngiid)) {
                        return Some(NumberError::BadGraphInputCleanup(*ngiid));
                    }
                }
            }
            NumberGraphEdit::AddGraphOutput(data) => {
                // the id must not be taken
                if topology.graph_output(data.id()).is_some() {
                    return Some(NumberError::GraphOutputIdTaken(data.id()));
                }

                // the output must not be connected
                if data.target().is_some() {
                    return Some(NumberError::BadGraphOutputInit(data.id()));
                }
            }
            NumberGraphEdit::RemoveGraphOutput(ngoid) => {
                // the output must exist
                let Some(data) = topology.graph_output(*ngoid) else {
                    return Some(NumberError::GraphOutputNotFound(*ngoid));
                };

                // the output must not be connected
                if data.target().is_some() {
                    return Some(NumberError::BadGraphOutputCleanup(*ngoid));
                }
            }
            NumberGraphEdit::ConnectNumberInput(niid, target) => {
                // the input must exist
                let Some(input_data) = topology.number_input(*niid) else {
                    return Some(NumberError::InputNotFound(*niid));
                };

                // the target must exist
                match *target {
                    NumberTarget::Source(nsid) => {
                        if topology.number_source(nsid).is_none() {
                            return Some(NumberError::SourceNotFound(nsid));
                        }
                    }
                    NumberTarget::GraphInput(ngiid) => {
                        if !topology.graph_inputs().contains(&ngiid) {
                            return Some(NumberError::GraphInputNotFound(ngiid));
                        }
                    }
                }

                // the input must be vacant
                if let Some(current_target) = input_data.target() {
                    return Some(NumberError::InputOccupied {
                        input_id: *niid,
                        current_target,
                    });
                }

                // the connection must be legal
                if let Err(e) = validate_number_connection(topology, *niid, *target) {
                    return Some(e);
                }
            }
            NumberGraphEdit::DisconnectNumberInput(niid) => {
                // the input must exist
                let Some(input_data) = topology.number_input(*niid) else {
                    return Some(NumberError::InputNotFound(*niid));
                };

                // the input must be occupied
                if input_data.target().is_none() {
                    return Some(NumberError::InputUnoccupied(*niid));
                }
            }
            NumberGraphEdit::ConnectGraphOutput(ngoid, target) => {
                // the output must exist
                let Some(output_data) = topology.graph_output(*ngoid) else {
                    return Some(NumberError::GraphOutputNotFound(*ngoid));
                };

                // the target must exist
                match *target {
                    NumberTarget::Source(nsid) => {
                        if topology.number_source(nsid).is_none() {
                            return Some(NumberError::SourceNotFound(nsid));
                        }
                    }
                    NumberTarget::GraphInput(ngiid) => {
                        if !topology.graph_inputs().contains(&ngiid) {
                            return Some(NumberError::GraphInputNotFound(ngiid));
                        }
                    }
                }

                // the output must be vacant
                if let Some(current_target) = output_data.target() {
                    return Some(NumberError::GraphOutputOccupied {
                        output_id: *ngoid,
                        current_target,
                    });
                }

                // NOTE: graph outputs can't be connected so as to create
                // a cycle, so no need to check against illegal connections
            }
            NumberGraphEdit::DisconnectGraphOutput(ngoid) => {
                // the output must exist
                let Some(output_data) = topology.graph_output(*ngoid) else {
                    return Some(NumberError::GraphOutputNotFound(*ngoid));
                };

                // the output must be occupied
                if output_data.target().is_none() {
                    return Some(NumberError::GraphOutputUnoccupied(*ngoid));
                }
            }
        }
        None
    }
}
