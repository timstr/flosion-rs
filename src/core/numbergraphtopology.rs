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
            NumberGraphEdit::AddGraphOutput(niid) => self.add_graph_output(niid),
            NumberGraphEdit::RemoveGraphOutput(niid) => self.remove_graph_output(niid),
            NumberGraphEdit::AddGraphInput(nsid) => self.add_graph_input(nsid),
            NumberGraphEdit::RemoveGraphInput(nsid) => self.remove_graph_input(nsid),
        }
        // TODO: validation
    }

    fn add_number_input(&mut self, data: NumberInputData) {
        debug_assert!(data.target().is_none());
        if let NumberInputOwner::NumberSource(source_id) = data.owner() {
            let source_data = self.number_sources.get_mut(&source_id).unwrap();
            source_data.number_inputs_mut().push(data.id());
        }
        let prev = self.number_inputs.insert(data.id(), data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_input(&mut self, input_id: NumberInputId, owner: NumberInputOwner) {
        todo!()
    }

    fn add_graph_output(&mut self, input_id: NumberInputId) {
        todo!()
    }

    fn remove_graph_output(&mut self, input_id: NumberInputId) {
        todo!()
    }

    fn add_graph_input(&mut self, source_id: NumberSourceId) {
        todo!()
    }

    fn remove_graph_input(&mut self, source_id: NumberSourceId) {
        todo!()
    }
}
