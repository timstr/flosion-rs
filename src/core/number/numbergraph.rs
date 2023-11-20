use std::sync::Arc;

use crate::core::{
    graph::{graph::Graph, graphobject::ObjectInitialization},
    number::{
        numbergraphvalidation::find_number_error, numbersource::PureNumberSourceWithId,
        numbersourcetools::NumberSourceTools,
    },
    uniqueid::{IdGenerator, UniqueId},
};

use super::{
    numbergraphdata::{NumberDestination, NumberGraphOutputData, NumberSourceData, NumberTarget},
    numbergraphedit::NumberGraphEdit,
    numbergrapherror::NumberError,
    numbergraphtopology::NumberGraphTopology,
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, PureNumberSource, PureNumberSourceHandle},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberGraphInputId(usize);

impl NumberGraphInputId {
    pub(crate) fn new(value: usize) -> NumberGraphInputId {
        NumberGraphInputId(value)
    }
}

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
pub struct NumberGraph {
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

    pub(crate) fn remove_graph_input(&mut self, id: NumberGraphInputId) -> Result<(), NumberError> {
        let mut edits = Vec::new();
        for ni in self.topology.number_inputs().values() {
            if ni.target() == Some(NumberTarget::GraphInput(id)) {
                edits.push(NumberGraphEdit::DisconnectNumberInput(ni.id()));
            }
        }
        for go in self.topology.graph_outputs() {
            if go.target() == Some(NumberTarget::GraphInput(id)) {
                edits.push(NumberGraphEdit::DisconnectGraphOutput(go.id()));
            }
        }
        edits.push(NumberGraphEdit::RemoveGraphInput(id));
        self.try_make_edits(edits)
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

    // Will be needed once multiple outputs are supported
    pub(crate) fn remove_graph_output(
        &mut self,
        id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        let mut edits = Vec::new();
        let Some(data) = self.topology.graph_output(id) else {
            return Err(NumberError::GraphOutputNotFound(id));
        };
        if data.target().is_some() {
            edits.push(NumberGraphEdit::DisconnectGraphOutput(id));
        }
        edits.push(NumberGraphEdit::RemoveGraphOutput(id));
        self.try_make_edits(edits)
    }

    pub fn add_number_source<T: PureNumberSource>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<PureNumberSourceHandle<T>, ()> {
        let id = self.number_source_idgen.next_id();
        let mut edit_queue = Vec::new();
        let source;
        {
            let tools = NumberSourceTools::new(id, &mut self.number_input_idgen, &mut edit_queue);
            source = Arc::new(PureNumberSourceWithId::new(T::new(tools, init)?, id));
        }
        let source2 = Arc::clone(&source);
        let data = NumberSourceData::new(id, source2);
        edit_queue.insert(0, NumberGraphEdit::AddNumberSource(data));
        self.try_make_edits(edit_queue).unwrap();
        Ok(PureNumberSourceHandle::new(source))
    }

    pub fn remove_number_source(&mut self, id: NumberSourceId) -> Result<(), NumberError> {
        let Some(data) = self.topology.number_source(id) else {
            return Err(NumberError::SourceNotFound(id));
        };
        let mut edits = Vec::new();
        for dst in self
            .topology
            .number_target_destinations(NumberTarget::Source(id))
        {
            let edit = match dst {
                NumberDestination::Input(niid) => NumberGraphEdit::DisconnectNumberInput(niid),
                NumberDestination::GraphOutput(goid) => {
                    NumberGraphEdit::DisconnectGraphOutput(goid)
                }
            };
            edits.push(edit);
        }
        for niid in data.number_inputs() {
            let Some(ni) = self.topology.number_input(*niid) else {
                return Err(NumberError::InputNotFound(*niid));
            };
            if ni.target().is_some() {
                edits.push(NumberGraphEdit::DisconnectNumberInput(*niid));
            }
            edits.push(NumberGraphEdit::RemoveNumberInput(*niid));
        }
        edits.push(NumberGraphEdit::RemoveNumberSource(id));
        self.try_make_edits(edits)
    }

    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        let edits = vec![NumberGraphEdit::ConnectNumberInput(input_id, target)];
        self.try_make_edits(edits)
    }

    pub fn disconnect_number_input(&mut self, input_id: NumberInputId) -> Result<(), NumberError> {
        let edits = vec![NumberGraphEdit::DisconnectNumberInput(input_id)];
        self.try_make_edits(edits)
    }

    pub fn connect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        let edits = vec![NumberGraphEdit::ConnectGraphOutput(output_id, target)];
        self.try_make_edits(edits)
    }

    pub fn disconnect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        let edits = vec![NumberGraphEdit::DisconnectGraphOutput(output_id)];
        self.try_make_edits(edits)
    }

    pub fn disconnect_destination(&mut self, target: NumberDestination) -> Result<(), NumberError> {
        let edit = match target {
            NumberDestination::Input(niid) => NumberGraphEdit::DisconnectNumberInput(niid),
            NumberDestination::GraphOutput(goid) => NumberGraphEdit::DisconnectGraphOutput(goid),
        };
        self.try_make_edits(vec![edit])
    }

    pub fn apply_number_source_tools<F: FnOnce(NumberSourceTools)>(
        &mut self,
        source_id: NumberSourceId,
        f: F,
    ) -> Result<(), NumberError> {
        let mut edit_queue = Vec::new();
        {
            let tools =
                NumberSourceTools::new(source_id, &mut self.number_input_idgen, &mut edit_queue);
            f(tools);
        }
        self.try_make_edits(edit_queue)
    }

    fn try_make_edits_locally(&mut self, edits: Vec<NumberGraphEdit>) -> Result<(), NumberError> {
        for edit in edits {
            if let Some(err) = edit.check_preconditions(&self.topology) {
                return Err(err);
            }
            self.topology.make_edit(edit);
            if let Some(err) = find_number_error(&self.topology) {
                return Err(err);
            }
        }
        Ok(())
    }

    fn try_make_edits(&mut self, edits: Vec<NumberGraphEdit>) -> Result<(), NumberError> {
        debug_assert!(find_number_error(&self.topology).is_none());
        let previous_topology = self.topology.clone();
        let res = self.try_make_edits_locally(edits);
        if res.is_err() {
            self.topology = previous_topology;
            return res;
        }
        Ok(())
    }
}

impl Graph for NumberGraph {
    type ObjectId = NumberSourceId;
}
