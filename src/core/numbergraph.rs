use super::{
    numbergraphdata::NumberGraphOutputData,
    numbergraphedit::NumberGraphEdit,
    numbergraphtopology::NumberGraphTopology,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    uniqueid::{IdGenerator, UniqueId},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberGraphInputId(usize);

impl Default for NumberGraphInputId {
    fn default() -> NumberGraphInputId {
        NumberGraphInputId(1)
    }
}

impl UniqueId for NumberGraphInputId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        NumberGraphInputId(self.0 + 1)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberGraphOutputId(usize);

impl Default for NumberGraphOutputId {
    fn default() -> NumberGraphOutputId {
        NumberGraphOutputId(1)
    }
}

impl UniqueId for NumberGraphOutputId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        NumberGraphOutputId(self.0 + 1)
    }
}

#[derive(Clone)]
pub(crate) struct NumberGraph {
    topology: NumberGraphTopology,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,
    graph_input_idgen: IdGenerator<NumberGraphInputId>,
    graph_output_idgen: IdGenerator<NumberGraphOutputId>,
}

impl NumberGraph {
    pub(crate) fn new() -> NumberGraph {
        NumberGraph {
            topology: NumberGraphTopology::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
            graph_input_idgen: IdGenerator::new(),
            graph_output_idgen: IdGenerator::new(),
        }
    }

    pub(crate) fn topology(&self) -> &NumberGraphTopology {
        &self.topology
    }

    pub(crate) fn add_graph_input(&mut self) -> NumberGraphInputId {
        let id = self.graph_input_idgen.next_id();
        self.topology.make_edit(NumberGraphEdit::AddGraphInput(id));
        id
    }

    pub(crate) fn remove_graph_input(&mut self, id: NumberGraphInputId) {
        self.topology
            .make_edit(NumberGraphEdit::RemoveGraphInput(id));
    }

    pub(crate) fn add_graph_output(&mut self, default_value: f32) -> NumberGraphOutputId {
        let id = self.graph_output_idgen.next_id();
        self.topology
            .make_edit(NumberGraphEdit::AddGraphOutput(NumberGraphOutputData::new(
                id,
                default_value,
            )));
        id
    }

    pub(crate) fn remove_graph_output(&mut self, id: NumberGraphOutputId) {
        self.topology
            .make_edit(NumberGraphEdit::RemoveGraphOutput(id));
    }

    // TODO: similar interface to SoundGraph for adding number sources and making connections

    // NOTE that number graphs never need to be sent to the audio thread!
    // Only pre-compiled artefacts for individual sound processor number inputs
    // need ever be sent. The input sub-graphs and top-level number functions
    // should have no other representation in the state graph on the audio thread.
}
