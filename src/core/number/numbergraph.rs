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

/// A unique integer identifier for a number graph input.
/// No two graph inputs in the same number graph may share
/// the same id.
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

/// A unique integer identifier for a number graph output.
/// No two graph outputs in the same number graph may share
/// the same id.
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

/// Convenience struct for passing the various id generators
/// used by a number graph together as a whole
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

/// A network of connected number sources which together represent
/// a computable expression over arrays of numbers. The number
/// graph is not directly executable and does not directly spawn
/// any worker threads, unlike SoundGraph. Instead, it must be
/// JIT-compiled in order to be evaluated. See the jit module.
/// Compared to operating directly on NumberGraphTopology,
/// NumberGraph provides a higher level interface and additional
/// error checking. Methods that modify the topology internally
/// will check the graph for validity, and if an errors are found,
/// edits to the topology will be rolled back automatically before
/// the error is forwarded.
#[derive(Clone)]
pub struct NumberGraph {
    topology: NumberGraphTopology,
    id_generators: NumberGraphIdGenerators,
}

impl NumberGraph {
    /// Creates a new NumberGraph with no sources and
    /// no graph inputs or outputs
    pub(crate) fn new() -> NumberGraph {
        NumberGraph {
            topology: NumberGraphTopology::new(),
            id_generators: NumberGraphIdGenerators::new(),
        }
    }

    /// Access the graph's topology.
    /// To modify the topology, see NumberGraph's other
    /// high-level methods
    pub(crate) fn topology(&self) -> &NumberGraphTopology {
        &self.topology
    }

    /// Add a graph input, through which the number graph
    /// receives numeric values from the outside. Internally,
    /// these can be connected to number sources, and they
    /// thus resemble function arguments in how they work.
    /// Graph inputs may also be directly connected to a
    /// graph output, which has the effect of returning
    /// that graph input unmodified when the graph is evaluated.
    /// The id of the newly added graph input is returned.
    pub(crate) fn add_graph_input(&mut self) -> NumberGraphInputId {
        let id = self.id_generators.graph_input.next_id();
        self.topology.add_graph_input(id).unwrap();
        id
    }

    /// Remove a graph input that was previously added with
    /// add_graph_input. Anything that the graph input is
    /// connected to will be disconnected.
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

    /// Add a graph output. Each graph output is a numeric
    /// value that is computed by the graph and is given
    /// back to the outside, and so it resembles a function's
    /// return value in how it works. Multiple graph outputs
    /// are permitted, and allow the same partial computations
    /// to be reused for multiple distinct but related results.
    /// A number graph with no graph outputs does not computed
    /// or evaluate anything and thus is not useful.
    /// The id of the newly created graph output is returned.
    pub(crate) fn add_graph_output(&mut self, default_value: f32) -> NumberGraphOutputId {
        let id = self.id_generators.graph_output.next_id();
        self.topology
            .add_graph_output(NumberGraphOutputData::new(id, default_value))
            .unwrap();
        id
    }

    /// Remove a graph output that was previously added
    /// with add_graph_output. If the output is connected
    /// to anything, it is disconnected first.
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

    /// Add a pure number source to the graph, i.e. one that
    /// has no side effects and only depends on its inputs.
    /// A pure number source does not have any state that
    /// it needs to allocate, initialize, persist, and restore
    /// between allocations, and thus is relatively efficient.
    /// The type must be known statically and given. For
    /// other ways of creating a number source, see ObjectFactory.
    /// Returns a handle to the number source.
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

    /// Add a stateful number source to the graph, i.e. one that
    /// does not always produce the same result and typically
    /// requires some additional memory overhead in order to
    /// store time-varying state between evaluations.
    /// The type must be known statically and given. For
    /// other ways of creating a number source, see ObjectFactory.
    /// Returns a handle to the number source.
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

    /// Remove a number source from the graph by its id.
    /// All inputs belonging to the number source are
    /// disconnected and then removed, and anything that
    /// is connected to the given number source is
    /// similarly disconnected before the number source
    /// is finally removed.
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

    /// Connect a number input (belonging to a number source)
    /// to the given number target (number source or graph input).
    /// The input must be unoccupied and the connection must
    /// be valid.
    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.connect_number_input(input_id, target))
    }

    /// Disconnect the given number input. The input
    /// must be occupied.
    pub fn disconnect_number_input(&mut self, input_id: NumberInputId) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.disconnect_number_input(input_id))
    }

    /// Connect the graph output to the given target
    /// (number graph input or number source). The graph
    /// output must be unoccupied.
    pub fn connect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
        target: NumberTarget,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.connect_graph_output(output_id, target))
    }

    /// Disconnect the given graph output. The graph output
    /// must be occupied.
    pub fn disconnect_graph_output(
        &mut self,
        output_id: NumberGraphOutputId,
    ) -> Result<(), NumberError> {
        self.try_make_change(|topo, _| topo.disconnect_graph_output(output_id))
    }

    /// Convenience method for disconnecting a number
    /// destination (e.g. graph output or number input)
    /// from its target, whatever that is.
    pub fn disconnect_destination(&mut self, target: NumberDestination) -> Result<(), NumberError> {
        // TODO: why is there no connect_destination?
        // ...and thus half as many methods here?
        self.try_make_change(|topo, _| match target {
            NumberDestination::Input(niid) => topo.disconnect_number_input(niid),
            NumberDestination::GraphOutput(goid) => topo.disconnect_graph_output(goid),
        })
    }

    /// Create a NumberGraphTools instance for making topological
    /// changes to a single number source and pass them to the
    /// provided closure. The caller is assumed to already have
    /// a handle to the number source in question.
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

    /// Internal helper method for editing the number graph's topology,
    /// detecting any errors during and after, rolling back the changes
    /// if any errors were found, and otherwise forwarding the result
    /// and persisting the change.
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
            return res;
        } else if let Some(e) = find_number_error(&self.topology) {
            self.topology = previous_topology;
            return Err(e);
        }
        res
    }
}

impl Graph for NumberGraph {
    type ObjectId = NumberSourceId;
}
