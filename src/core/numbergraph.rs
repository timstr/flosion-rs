use super::{
    numbergraphedit::NumberGraphEdit, numbergraphtopology::NumberGraphTopology,
    numberinput::NumberInputId, numbersource::NumberSourceId, uniqueid::IdGenerator,
};

#[derive(Clone)]
pub(crate) struct NumberGraph {
    topology: NumberGraphTopology,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,
}

impl NumberGraph {
    pub(crate) fn new() -> NumberGraph {
        NumberGraph {
            topology: NumberGraphTopology::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
        }
    }

    pub(crate) fn topology(&self) -> &NumberGraphTopology {
        &self.topology
    }

    pub(crate) fn add_graph_input(&mut self) -> NumberSourceId {
        let id = self.number_source_idgen.next_id();
        self.topology.make_edit(NumberGraphEdit::AddGraphInput(id));
        id
    }

    pub(crate) fn remove_graph_input(&mut self, id: NumberSourceId) {
        self.topology
            .make_edit(NumberGraphEdit::RemoveGraphInput(id));
    }

    pub(crate) fn add_graph_output(&mut self) -> NumberInputId {
        let id = self.number_input_idgen.next_id();
        self.topology.make_edit(NumberGraphEdit::AddGraphOutput(id));
        id
    }

    pub(crate) fn remove_graph_output(&mut self, id: NumberInputId) {
        self.topology
            .make_edit(NumberGraphEdit::RemoveGraphOutput(id));
    }

    // TODO: similar interface to SoundGraph for adding number sources and making connections

    // NOTE that number graphs never need to be sent to the audio thread!
    // Only pre-compiled artefacts for individual sound processor number inputs
    // need ever be sent. The input sub-graphs and top-level number functions
    // should have no other representation in the state graph on the audio thread.
}
