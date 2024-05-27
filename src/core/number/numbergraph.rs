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
    numbergrapherror::NumberError,
    numbergraphtopology::NumberGraphTopology,
    numberinput::NumberInputId,
    numbersource::{
        NumberSourceId, PureNumberSource, PureNumberSourceHandle, StatefulNumberSource,
        StatefulNumberSourceHandle, StatefulNumberSourceWithId,
    },
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

#[derive(Clone, Copy)]
pub(crate) struct NumberGraphIdGenerators {
    pub number_source: IdGenerator<NumberSourceId>,
    pub number_input: IdGenerator<NumberInputId>,
    pub graph_input: IdGenerator<NumberGraphInputId>,
    pub graph_output: IdGenerator<NumberGraphOutputId>,
}

impl NumberGraphIdGenerators {
    pub(crate) fn new() -> NumberGraphIdGenerators {
        NumberGraphIdGenerators {
            number_source: IdGenerator::new(),
            number_input: IdGenerator::new(),
            graph_input: IdGenerator::new(),
            graph_output: IdGenerator::new(),
        }
    }
}

#[derive(Clone)]
pub struct NumberGraph {
    topology: NumberGraphTopology,
    id_generators: NumberGraphIdGenerators,
}

impl NumberGraph {
    pub(crate) fn new() -> NumberGraph {
        NumberGraph {
            topology: NumberGraphTopology::new(),
            id_generators: NumberGraphIdGenerators::new(),
        }
    }

    pub(crate) fn topology(&self) -> &NumberGraphTopology {
        &self.topology
    }

    pub(crate) fn add_graph_input(&mut self) -> NumberGraphInputId {
        let id = self.id_generators.graph_input.next_id();
        self.topology.add_graph_input(id).unwrap();
        id
    }

    pub(crate) fn remove_graph_input(&mut self, id: NumberGraphInputId) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| {
            let things_to_disconnect: Vec<_> = topo.number_target_destinations(id.into()).collect();

            for id in things_to_disconnect {
                match id {
                    NumberDestination::Input(niid) => topo.disconnect_number_input(niid)?,
                    NumberDestination::GraphOutput(goid) => topo.disconnect_graph_output(goid)?,
                }
            }

            topo.remove_graph_input(id)
        })
    }

    pub(crate) fn add_graph_output(&mut self, default_value: f32) -> NumberGraphOutputId {
        let id = self.id_generators.graph_output.next_id();
        self.topology
            .add_graph_output(NumberGraphOutputData::new(id, default_value))
            .unwrap();
        id
    }

    // Will be needed once multiple outputs are supported
    pub(crate) fn remove_graph_output(
        &mut self,
        id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| {
            let data = topo
                .graph_output(id)
                .ok_or(NumberError::GraphOutputNotFound(id))?;
            if data.target().is_some() {
                topo.disconnect_graph_output(id)?;
            }
            topo.remove_graph_output(id)
        })
    }

    pub fn add_pure_number_source<T: PureNumberSource>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<PureNumberSourceHandle<T>, NumberError> {
        self.try_make_change(|topo, idgens| {
            let id = idgens.number_source.next_id();
            topo.add_number_source(NumberSourceData::new_empty(id))?;
            let tools = NumberSourceTools::new(id, topo, idgens);
            let source = Arc::new(PureNumberSourceWithId::new(
                T::new(tools, init).map_err(|_| NumberError::BadSourceInit(id))?,
                id,
            ));
            let source2 = Arc::clone(&source);
            topo.number_source_mut(id).unwrap().set_instance(source);
            Ok(PureNumberSourceHandle::new(source2))
        })
    }

    pub fn add_stateful_number_source<T: StatefulNumberSource>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<StatefulNumberSourceHandle<T>, NumberError> {
        self.try_make_change(|topo, idgens| {
            let id = idgens.number_source.next_id();
            topo.add_number_source(NumberSourceData::new_empty(id))?;
            let tools = NumberSourceTools::new(id, topo, idgens);
            let source = Arc::new(StatefulNumberSourceWithId::new(
                T::new(tools, init).map_err(|_| NumberError::BadSourceInit(id))?,
                id,
            ));
            let source2 = Arc::clone(&source);
            topo.number_source_mut(id).unwrap().set_instance(source);
            Ok(StatefulNumberSourceHandle::new(source2))
        })
    }

    pub fn remove_number_source(&mut self, id: NumberSourceId) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| {
            let mut number_inputs_to_remove = Vec::new();
            let mut number_inputs_to_disconnect = Vec::new();
            let mut graph_outputs_to_disconnect = Vec::new();

            let source = topo
                .number_source(id)
                .ok_or(NumberError::SourceNotFound(id))?;

            for ni in source.number_inputs() {
                number_inputs_to_remove.push(*ni);
                if topo.number_input(*ni).unwrap().target().is_some() {
                    number_inputs_to_disconnect.push(*ni);
                }
            }

            for ni in topo.number_inputs().values() {
                if ni.target() == Some(NumberTarget::Source(id)) {
                    number_inputs_to_disconnect.push(ni.id());
                }
            }

            for go in topo.graph_outputs() {
                if go.target() == Some(NumberTarget::Source(id)) {
                    graph_outputs_to_disconnect.push(go.id());
                }
            }

            // ---

            for ni in number_inputs_to_disconnect {
                topo.disconnect_number_input(ni)?;
            }

            for go in graph_outputs_to_disconnect {
                topo.disconnect_graph_output(go)?;
            }

            for ni in number_inputs_to_remove {
                topo.remove_number_input(ni)?;
            }

            topo.remove_number_source(id)
        })
    }

    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.connect_number_input(input_id, target))
    }

    pub fn disconnect_number_input(&mut self, input_id: NumberInputId) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.disconnect_number_input(input_id))
    }

    pub fn connect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.connect_graph_output(output_id, target))
    }

    pub fn disconnect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.disconnect_graph_output(output_id))
    }

    pub fn disconnect_destination(&mut self, target: NumberDestination) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| match target {
            NumberDestination::Input(niid) => topo.disconnect_number_input(niid),
            NumberDestination::GraphOutput(goid) => topo.disconnect_graph_output(goid),
        })
    }

    pub fn apply_number_source_tools<F: FnOnce(NumberSourceTools)>(
        &mut self,
        source_id: NumberSourceId,
        f: F,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, idgens| {
            let tools = NumberSourceTools::new(source_id, topo, idgens);
            f(tools);
            Ok(())
        })
    }

    fn try_make_change<
        R,
        F: FnOnce(&mut NumberGraphTopology, &mut NumberGraphIdGenerators) -> Result<R, NumberError>,
    >(
        &mut self,
        f: F,
    ) -> Result<R, NumberError> {
        debug_assert!(find_number_error(&self.topology).is_none());
        let previous_topology = self.topology.clone();
        let res = f(&mut self.topology, &mut self.id_generators);
        if res.is_err() {
            self.topology = previous_topology;
        }
        res
    }
}

impl Graph for NumberGraph {
    type ObjectId = NumberSourceId;
}
