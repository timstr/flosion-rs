use std::collections::HashMap;

use super::{
    numbergraphdata::{NumberInputData, NumberSourceData},
    numbergraphedit::NumberGraphEdit,
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::NumberSourceId,
};

#[derive(Clone)]
pub(crate) struct NumberGraphTopology {
    number_sources: HashMap<NumberSourceId, NumberSourceData>,
    number_inputs: HashMap<NumberInputId, NumberInputData>,
    graph_inputs: Vec<NumberSourceId>,
    graph_outputs: Vec<NumberInputId>,
}

impl NumberGraphTopology {
    pub(crate) fn new() -> NumberGraphTopology {
        NumberGraphTopology {
            number_sources: HashMap::new(),
            number_inputs: HashMap::new(),
            graph_inputs: Vec::new(),
            graph_outputs: Vec::new(),
        }
    }

    pub(crate) fn number_input(&self, id: NumberInputId) -> Option<&NumberInputData> {
        self.number_inputs.get(&id)
    }

    pub(crate) fn number_source(&self, id: NumberSourceId) -> Option<&NumberSourceData> {
        self.number_sources.get(&id)
    }

    pub(crate) fn graph_inputs(&self) -> &[NumberSourceId] {
        &self.graph_inputs
    }

    pub(crate) fn graph_outputs(&self) -> &[NumberInputId] {
        &self.graph_outputs
    }

    pub(crate) fn make_edit(&mut self, edit: NumberGraphEdit) {
        // TODO: validation
        // TODO: precondition check
        match edit {
            NumberGraphEdit::AddNumberInput(data) => self.add_number_input(data),
            NumberGraphEdit::RemoveNumberInput(niid, owner) => {
                self.remove_number_input(niid, owner)
            }
            NumberGraphEdit::AddNumberSource(data) => todo!(),
            NumberGraphEdit::RemoveNumberSource(nsid) => todo!(),
        }
        // TODO: validation
    }

    fn add_number_input(&mut self, data: NumberInputData) {
        debug_assert!(data.target().is_none());
        match data.owner() {
            NumberInputOwner::NumberSource(source_id) => {
                let source_data = self.number_sources.get_mut(&source_id).unwrap();
                match source_data {
                    NumberSourceData::Instance(inst) => {
                        let inst_inputs = inst.number_inputs_mut();
                        debug_assert!(!inst_inputs.contains(&data.id()));
                        inst_inputs.push(data.id());
                    }
                    NumberSourceData::GraphInput(_) => panic!(),
                }
            }
            NumberInputOwner::ParentGraph => {
                debug_assert!(!self.graph_outputs.contains(&data.id()));
                self.graph_outputs.push(data.id());
            }
        }
        let prev = self.number_inputs.insert(data.id(), data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_input(&mut self, input_id: NumberInputId, owner: NumberInputOwner) {
        todo!()
    }

    fn add_number_source(&mut self, data: NumberSourceData) {
        match &data {
            NumberSourceData::Instance(inst_data) => {
                debug_assert!(inst_data.number_inputs().is_empty());
            }
            NumberSourceData::GraphInput(nsid) => {
                debug_assert!(!self.graph_inputs.contains(&nsid))
            }
        }
        let prev = self.number_sources.insert(data.id(), data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_source(&mut self, source_id: NumberSourceId) {
        todo!()
    }
}
