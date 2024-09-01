use std::sync::Arc;

use crate::{
    core::{
        expression::expressiongraphvalidation::find_expression_error,
        graph::graph::Graph,
        uniqueid::{IdGenerator, UniqueId},
    },
    ui_core::arguments::ParsedArguments,
};

use super::{
    expressiongraphdata::{
        ExpressionDestination, ExpressionGraphResultData, ExpressionNodeData, ExpressionTarget,
    },
    expressiongrapherror::ExpressionError,
    expressiongraphtopology::ExpressionGraphTopology,
    expressionnode::{
        ExpressionNodeId, PureExpressionNode, PureExpressionNodeHandle, PureExpressionNodeWithId,
        StatefulExpressionNode, StatefulExpressionNodeHandle, StatefulExpressionNodeWithId,
    },
    expressionnodeinput::ExpressionNodeInputId,
    expressionnodetools::ExpressionNodeTools,
};

pub struct ExpressionGraphParameterTag;

/// A unique integer identifier for an expression graph parameter.
/// No two parameters of the same expression graph may share
/// the same id.
pub type ExpressionGraphParameterId = UniqueId<ExpressionGraphParameterTag>;

pub struct ExpressionGraphResultTag;

/// A unique integer identifier for an expression graph result.
/// No two results of the same expression graph may share
/// the same id.
pub type ExpressionGraphResultId = UniqueId<ExpressionGraphResultTag>;

/// Convenience struct for passing the various id generators
/// used by a expression graph together as a whole
#[derive(Clone, Copy)]
pub(crate) struct ExpressionGraphIdGenerators {
    pub node: IdGenerator<ExpressionNodeId>,
    pub node_input: IdGenerator<ExpressionNodeInputId>,
    pub parameter: IdGenerator<ExpressionGraphParameterId>,
    pub result: IdGenerator<ExpressionGraphResultId>,
}

impl ExpressionGraphIdGenerators {
    pub(crate) fn new() -> ExpressionGraphIdGenerators {
        ExpressionGraphIdGenerators {
            node: IdGenerator::new(),
            node_input: IdGenerator::new(),
            parameter: IdGenerator::new(),
            result: IdGenerator::new(),
        }
    }
}

/// A network of connected expression nodes which together represent
/// a computable expression over arrays of numbers. The expression
/// graph is not directly executable and does not directly spawn
/// any worker threads, unlike SoundGraph. Instead, it must be
/// JIT-compiled in order to be evaluated. See the jit module.
/// Compared to operating directly on ExpressionGraphTopology,
/// ExpressionGraph provides a higher level interface and additional
/// error checking. Methods that modify the topology internally
/// will check the graph for validity, and if an errors are found,
/// edits to the topology will be rolled back automatically before
/// the error is forwarded.
#[derive(Clone)]
pub struct ExpressionGraph {
    topology: ExpressionGraphTopology,
    id_generators: ExpressionGraphIdGenerators,
}

impl ExpressionGraph {
    /// Creates a new ExpressionGraph with no nodes and
    /// no graph parameters or results
    pub(crate) fn new() -> ExpressionGraph {
        ExpressionGraph {
            topology: ExpressionGraphTopology::new(),
            id_generators: ExpressionGraphIdGenerators::new(),
        }
    }

    /// Access the graph's topology.
    /// To modify the topology, see ExpressionGraph's other
    /// high-level methods
    pub(crate) fn topology(&self) -> &ExpressionGraphTopology {
        &self.topology
    }

    /// Add a parameter, through which the expression graph
    /// receives numeric values from the outside. Internally,
    /// these can be connected to expression nodes, and they
    /// thus resemble function parameters in how they work.
    /// Graph parameters may also be directly connected to a
    /// graph output, which has the effect of returning
    /// that graph input unmodified when the graph is evaluated.
    /// The id of the newly added graph input is returned.
    pub(crate) fn add_parameter(&mut self) -> ExpressionGraphParameterId {
        let id = self.id_generators.parameter.next_id();
        self.topology.add_parameter(id).unwrap();
        id
    }

    /// Remove a graph input that was previously added with
    /// add_parameter. Anything that the graph input is
    /// connected to will be disconnected.
    pub(crate) fn remove_parameter(
        &mut self,
        id: ExpressionGraphParameterId,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| {
            let things_to_disconnect: Vec<_> = topo.destinations(id.into()).collect();

            for id in things_to_disconnect {
                match id {
                    ExpressionDestination::NodeInput(niid) => topo.disconnect_node_input(niid)?,
                    ExpressionDestination::Result(goid) => topo.disconnect_result(goid)?,
                }
            }

            topo.remove_parameter(id)
        })
    }

    /// Add an expression result. Each result is a numeric
    /// value that is computed by the graph and is given
    /// back to the outside, and so it resembles a function's
    /// return value in how it works. Multiple results
    /// are permitted, and allow the same partial computations
    /// to be reused for multiple distinct but related results.
    /// An expression graph with no results does not compute
    /// or evaluate anything and thus is not useful.
    /// The id of the newly created result is returned.
    pub(crate) fn add_result(&mut self, default_value: f32) -> ExpressionGraphResultId {
        let id = self.id_generators.result.next_id();
        self.topology
            .add_result(ExpressionGraphResultData::new(id, default_value))
            .unwrap();
        id
    }

    /// Remove an expression graph result that was previously added
    /// with add_result. If the result is connected to anything, it
    /// is disconnected first.
    pub(crate) fn remove_result(
        &mut self,
        id: ExpressionGraphResultId,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| {
            let data = topo.result(id).ok_or(ExpressionError::ResultNotFound(id))?;
            if data.target().is_some() {
                topo.disconnect_result(id)?;
            }
            topo.remove_result(id)
        })
    }

    /// Add a pure expression node to the graph, i.e. one that
    /// has no side effects and only depends on its inputs.
    /// A pure expression node does not have any state that
    /// it needs to allocate, initialize, persist, and restore
    /// between allocations, and thus is relatively efficient.
    /// The type must be known statically and given. For
    /// other ways of creating an expression node, see ObjectFactory.
    /// Returns a handle to the expression node.
    pub fn add_pure_expression_node<T: 'static + PureExpressionNode>(
        &mut self,
        args: ParsedArguments,
    ) -> Result<PureExpressionNodeHandle<T>, ExpressionError> {
        self.try_make_change(|topo, idgens| {
            let id = idgens.node.next_id();
            topo.add_node(ExpressionNodeData::new_empty(id))?;
            let tools = ExpressionNodeTools::new(id, topo, idgens);
            let node = Arc::new(PureExpressionNodeWithId::new(
                T::new(tools, args).map_err(|_| ExpressionError::BadNodeInit(id))?,
                id,
            ));
            let node2 = Arc::clone(&node);
            topo.node_mut(id).unwrap().set_instance(node);
            Ok(PureExpressionNodeHandle::new(node2))
        })
    }

    /// Add a stateful expression node to the graph, i.e. one that
    /// does not always produce the same result and typically
    /// requires some additional memory overhead in order to
    /// store time-varying state between evaluations.
    /// The type must be known statically and given. For
    /// other ways of creating an expression node, see ObjectFactory.
    /// Returns a handle to the expression node.
    pub fn add_stateful_expression_node<T: 'static + StatefulExpressionNode>(
        &mut self,
        args: ParsedArguments,
    ) -> Result<StatefulExpressionNodeHandle<T>, ExpressionError> {
        self.try_make_change(|topo, idgens| {
            let id = idgens.node.next_id();
            topo.add_node(ExpressionNodeData::new_empty(id))?;
            let tools = ExpressionNodeTools::new(id, topo, idgens);
            let node = Arc::new(StatefulExpressionNodeWithId::new(
                T::new(tools, args).map_err(|_| ExpressionError::BadNodeInit(id))?,
                id,
            ));
            let node2 = Arc::clone(&node);
            topo.node_mut(id).unwrap().set_instance(node);
            Ok(StatefulExpressionNodeHandle::new(node2))
        })
    }

    /// Remove an expression node from the graph by its id.
    /// All inputs belonging to the expression node are
    /// disconnected and then removed, and anything that
    /// is connected to the given expression node is
    /// similarly disconnected before the expression node
    /// itself is finally removed.
    pub fn remove_expression_node(&mut self, id: ExpressionNodeId) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| {
            let mut inputs_to_remove = Vec::new();
            let mut inputs_to_disconnect = Vec::new();
            let mut results_to_disconnect = Vec::new();

            let node = topo.node(id).ok_or(ExpressionError::NodeNotFound(id))?;

            for ni in node.inputs() {
                inputs_to_remove.push(*ni);
                if topo.node_input(*ni).unwrap().target().is_some() {
                    inputs_to_disconnect.push(*ni);
                }
            }

            for ni in topo.node_inputs().values() {
                if ni.target() == Some(ExpressionTarget::Node(id)) {
                    inputs_to_disconnect.push(ni.id());
                }
            }

            for go in topo.results() {
                if go.target() == Some(ExpressionTarget::Node(id)) {
                    results_to_disconnect.push(go.id());
                }
            }

            // ---

            for ni in inputs_to_disconnect {
                topo.disconnect_node_input(ni)?;
            }

            for go in results_to_disconnect {
                topo.disconnect_result(go)?;
            }

            for ni in inputs_to_remove {
                topo.remove_node_input(ni)?;
            }

            topo.remove_node(id)
        })
    }

    /// Connect an expression node input
    /// to the given target (expression node or graph parameter).
    /// The input must be unoccupied and the connection must
    /// be valid.
    pub fn connect_node_input(
        &mut self,
        input_id: ExpressionNodeInputId,
        target: ExpressionTarget,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| topo.connect_node_input(input_id, target))
    }

    /// Disconnect the given expression node input. The input
    /// must be occupied.
    pub fn disconnect_node_input(
        &mut self,
        input_id: ExpressionNodeInputId,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| topo.disconnect_node_input(input_id))
    }

    /// Connect the graph result to the given target
    /// (expression node or graph parameter). The graph
    /// result must be unoccupied.
    pub fn connect_result(
        &mut self,
        output_id: ExpressionGraphResultId,
        target: ExpressionTarget,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| topo.connect_result(output_id, target))
    }

    /// Disconnect the given graph result. The graph result
    /// must be occupied.
    pub fn disconnect_result(
        &mut self,
        output_id: ExpressionGraphResultId,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, _| topo.disconnect_result(output_id))
    }

    /// Create a ExpressionNodeTools instance for making topological
    /// changes to a single expression node pass them to the
    /// provided closure. The caller is assumed to already have
    /// a handle to the expression node in question.
    pub fn apply_expression_node_tools<F: FnOnce(ExpressionNodeTools)>(
        &mut self,
        node_id: ExpressionNodeId,
        f: F,
    ) -> Result<(), ExpressionError> {
        self.try_make_change(|topo, idgens| {
            let tools = ExpressionNodeTools::new(node_id, topo, idgens);
            f(tools);
            Ok(())
        })
    }

    /// Internal helper method for editing the expression graph's topology,
    /// detecting any errors during and after, rolling back the changes
    /// if any errors were found, and otherwise forwarding the result
    /// and persisting the change.
    fn try_make_change<
        R,
        F: FnOnce(
            &mut ExpressionGraphTopology,
            &mut ExpressionGraphIdGenerators,
        ) -> Result<R, ExpressionError>,
    >(
        &mut self,
        f: F,
    ) -> Result<R, ExpressionError> {
        debug_assert!(find_expression_error(&self.topology).is_none());
        let previous_topology = self.topology.clone();
        let res = f(&mut self.topology, &mut self.id_generators);
        if res.is_err() {
            self.topology = previous_topology;
            return res;
        } else if let Some(e) = find_expression_error(&self.topology) {
            self.topology = previous_topology;
            return Err(e);
        }
        res
    }
}

impl Graph for ExpressionGraph {
    type ObjectId = ExpressionNodeId;
}
