use std::{collections::HashMap, hash::Hasher};

use crate::core::{revision::Revision, uniqueid::UniqueId};

use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numbergraphdata::{
        NumberDestination, NumberGraphOutputData, NumberInputData, NumberSourceData, NumberTarget,
    },
    numbergraphedit::NumberGraphEdit,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
};

#[derive(Clone)]
pub(crate) struct NumberGraphTopology {
    number_sources: HashMap<NumberSourceId, NumberSourceData>,
    number_inputs: HashMap<NumberInputId, NumberInputData>,
    graph_inputs: Vec<NumberGraphInputId>,
    graph_outputs: Vec<NumberGraphOutputData>,
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

    pub(crate) fn number_inputs(&self) -> &HashMap<NumberInputId, NumberInputData> {
        &self.number_inputs
    }

    pub(crate) fn number_sources(&self) -> &HashMap<NumberSourceId, NumberSourceData> {
        &self.number_sources
    }

    pub(crate) fn graph_inputs(&self) -> &[NumberGraphInputId] {
        &self.graph_inputs
    }

    pub(crate) fn graph_output(&self, id: NumberGraphOutputId) -> Option<&NumberGraphOutputData> {
        self.graph_outputs.iter().filter(|x| x.id() == id).next()
    }

    pub(crate) fn graph_outputs(&self) -> &[NumberGraphOutputData] {
        &self.graph_outputs
    }

    pub(crate) fn number_target_destinations<'a>(
        &'a self,
        target: NumberTarget,
    ) -> impl 'a + Iterator<Item = NumberDestination> {
        let matching_number_inputs = self.number_inputs.values().filter_map(move |i| {
            if i.target() == Some(target) {
                Some(NumberDestination::Input(i.id()))
            } else {
                None
            }
        });
        let matching_graph_outputs = self.graph_outputs.iter().filter_map(move |i| {
            if i.target() == Some(target) {
                Some(NumberDestination::GraphOutput(i.id()))
            } else {
                None
            }
        });
        matching_number_inputs.chain(matching_graph_outputs)
    }

    pub(crate) fn make_edit(&mut self, edit: NumberGraphEdit) {
        match edit {
            NumberGraphEdit::AddNumberInput(data) => self.add_number_input(data),
            NumberGraphEdit::RemoveNumberInput(niid) => self.remove_number_input(niid),
            NumberGraphEdit::AddNumberSource(data) => self.add_number_source(data),
            NumberGraphEdit::RemoveNumberSource(nsid) => self.remove_number_source(nsid),
            NumberGraphEdit::ConnectNumberInput(niid, tid) => self.connect_number_input(niid, tid),
            NumberGraphEdit::DisconnectNumberInput(niid) => self.disconnect_number_input(niid),
            NumberGraphEdit::AddGraphInput(data) => self.add_graph_input(data),
            NumberGraphEdit::RemoveGraphInput(giid) => self.remove_graph_input(giid),
            NumberGraphEdit::AddGraphOutput(goid) => self.add_graph_output(goid),
            NumberGraphEdit::RemoveGraphOutput(goid) => self.remove_graph_output(goid),
            NumberGraphEdit::ConnectGraphOutput(goid, tid) => self.connect_graph_output(goid, tid),
            NumberGraphEdit::DisconnectGraphOutput(goid) => self.disconnect_graph_output(goid),
        }
    }

    fn add_number_input(&mut self, data: NumberInputData) {
        debug_assert!(data.target().is_none());
        let ns_data = self.number_sources.get_mut(&data.owner()).unwrap();
        debug_assert!(!ns_data.number_inputs().contains(&data.id()));
        ns_data.number_inputs_mut().push(data.id());
        let prev = self.number_inputs.insert(data.id(), data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_input(&mut self, input_id: NumberInputId) {
        debug_assert!(self.number_input(input_id).unwrap().target().is_none());
        let owner = self.number_input(input_id).unwrap().owner();
        let ns_data = self.number_sources.get_mut(&owner).unwrap();
        debug_assert_eq!(
            ns_data
                .number_inputs()
                .iter()
                .filter(|x| **x == input_id)
                .count(),
            1
        );
        ns_data.number_inputs_mut().retain(|x| *x != input_id);
        let prev = self.number_inputs.remove(&input_id);
        debug_assert!(prev.is_some());
    }

    fn add_number_source(&mut self, data: NumberSourceData) {
        debug_assert!(data.number_inputs().is_empty());
        let prev = self.number_sources.insert(data.id(), data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_source(&mut self, source_id: NumberSourceId) {
        debug_assert!(!self.number_inputs.values().any(|d| d.owner() == source_id));
        debug_assert_eq!(self.number_target_destinations(source_id.into()).count(), 0);
        debug_assert!(self
            .number_sources
            .get(&source_id)
            .unwrap()
            .number_inputs()
            .is_empty());
        let prev = self.number_sources.remove(&source_id);
        debug_assert!(prev.is_some());
    }

    fn connect_number_input(&mut self, input_id: NumberInputId, target: NumberTarget) {
        debug_assert!(match target {
            NumberTarget::Source(nsid) => self.number_sources.contains_key(&nsid),
            NumberTarget::GraphInput(giid) => self.graph_inputs.contains(&giid),
        });
        let data = self.number_inputs.get_mut(&input_id).unwrap();
        debug_assert!(data.target().is_none());
        data.set_target(Some(target));
    }

    fn disconnect_number_input(&mut self, input_id: NumberInputId) {
        let data = self.number_inputs.get_mut(&input_id).unwrap();
        debug_assert!(data.target().is_some());
        data.set_target(None);
    }

    fn add_graph_input(&mut self, input_id: NumberGraphInputId) {
        debug_assert!(!self.graph_inputs.contains(&input_id));
        self.graph_inputs.push(input_id);
    }

    fn remove_graph_input(&mut self, input_id: NumberGraphInputId) {
        debug_assert_eq!(
            self.graph_inputs.iter().filter(|x| **x == input_id).count(),
            1
        );
        debug_assert!(!self
            .number_inputs
            .values()
            .any(|x| x.target() == Some(NumberTarget::GraphInput(input_id))));
        debug_assert!(!self
            .graph_outputs
            .iter()
            .any(|x| x.target() == Some(NumberTarget::GraphInput(input_id))));
        self.graph_inputs.retain(|x| *x != input_id);
    }

    fn add_graph_output(&mut self, data: NumberGraphOutputData) {
        debug_assert!(data.target().is_none());
        debug_assert_eq!(
            self.graph_outputs
                .iter()
                .filter(|x| x.id() == data.id())
                .count(),
            0
        );
        self.graph_outputs.push(data);
    }

    fn remove_graph_output(&mut self, output_id: NumberGraphOutputId) {
        debug_assert_eq!(
            self.graph_outputs
                .iter()
                .filter(|x| x.id() == output_id)
                .count(),
            1
        );
        debug_assert!(self
            .graph_outputs
            .iter()
            .filter(|x| x.id() == output_id)
            .next()
            .unwrap()
            .target()
            .is_none());
        self.graph_outputs.retain(|x| x.id() != output_id);
    }

    fn connect_graph_output(&mut self, output_id: NumberGraphOutputId, target: NumberTarget) {
        debug_assert!(match target {
            NumberTarget::Source(nsid) => self.number_sources.contains_key(&nsid),
            NumberTarget::GraphInput(giid) => self.graph_inputs.contains(&giid),
        });
        let data = self
            .graph_outputs
            .iter_mut()
            .filter(|x| x.id() == output_id)
            .next()
            .unwrap();
        debug_assert!(data.target().is_none());
        data.set_target(Some(target));
    }

    fn disconnect_graph_output(&mut self, output_id: NumberGraphOutputId) {
        let data = self
            .graph_outputs
            .iter_mut()
            .filter(|x| x.id() == output_id)
            .next()
            .unwrap();
        debug_assert!(data.target().is_some());
        data.set_target(None);
    }
}

impl Revision for NumberGraphTopology {
    fn get_revision(&self) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u64(self.number_sources.get_revision());
        hasher.write_u64(self.number_inputs.get_revision());
        hasher.write_usize(self.graph_inputs.len());
        for giid in &self.graph_inputs {
            hasher.write_usize(giid.value());
        }
        hasher.write_usize(self.graph_outputs.len());
        for o in &self.graph_outputs {
            hasher.write_u64(o.get_revision());
        }
        hasher.finish()
    }
}
