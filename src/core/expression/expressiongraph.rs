use std::{collections::HashMap, rc::Rc};

use crate::{
    core::{expression::expressiongraphvalidation::find_expression_error, uniqueid::UniqueId},
    ui_core::arguments::ParsedArguments,
};

use super::{
    expressiongraphdata::{
        ExpressionDestination, ExpressionGraphResultData, ExpressionNodeData,
        ExpressionNodeInputData, ExpressionTarget,
    },
    expressiongrapherror::ExpressionError,
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
pub(crate) struct ExpressionGraphIdGenerators {}

#[derive(Clone)]
pub struct ExpressionGraph {
    nodes: HashMap<ExpressionNodeId, ExpressionNodeData>,
    node_inputs: HashMap<ExpressionNodeInputId, ExpressionNodeInputData>,
    parameters: Vec<ExpressionGraphParameterId>,
    results: Vec<ExpressionGraphResultData>,
}

impl ExpressionGraph {
    pub(crate) fn new() -> ExpressionGraph {
        ExpressionGraph {
            nodes: HashMap::new(),
            node_inputs: HashMap::new(),
            parameters: Vec::new(),
            results: Vec::new(),
        }
    }

    pub(crate) fn node_input(&self, id: ExpressionNodeInputId) -> Option<&ExpressionNodeInputData> {
        self.node_inputs.get(&id)
    }

    pub(crate) fn node(&self, id: ExpressionNodeId) -> Option<&ExpressionNodeData> {
        self.nodes.get(&id)
    }

    pub(crate) fn node_inputs(&self) -> &HashMap<ExpressionNodeInputId, ExpressionNodeInputData> {
        &self.node_inputs
    }

    pub(crate) fn nodes(&self) -> &HashMap<ExpressionNodeId, ExpressionNodeData> {
        &self.nodes
    }

    pub(crate) fn parameters(&self) -> &[ExpressionGraphParameterId] {
        &self.parameters
    }

    pub(crate) fn result(&self, id: ExpressionGraphResultId) -> Option<&ExpressionGraphResultData> {
        self.results.iter().filter(|x| x.id() == id).next()
    }

    pub(crate) fn results(&self) -> &[ExpressionGraphResultData] {
        &self.results
    }

    pub(crate) fn destinations<'a>(
        &'a self,
        target: ExpressionTarget,
    ) -> impl 'a + Iterator<Item = ExpressionDestination> {
        let matching_inputs = self.node_inputs.values().filter_map(move |i| {
            if i.target() == Some(target) {
                Some(ExpressionDestination::NodeInput(i.id()))
            } else {
                None
            }
        });
        let matching_results = self.results.iter().filter_map(move |i| {
            if i.target() == Some(target) {
                Some(ExpressionDestination::Result(i.id()))
            } else {
                None
            }
        });
        matching_inputs.chain(matching_results)
    }

    pub fn add_node_input(
        &mut self,
        owner: ExpressionNodeId,
        default_value: f32,
    ) -> Result<ExpressionNodeInputId, ExpressionError> {
        if self.node(owner).is_none() {
            return Err(ExpressionError::NodeNotFound(owner));
        }

        let id = ExpressionNodeInputId::new_unique();
        let data = ExpressionNodeInputData::new(id, owner, default_value);

        let ns_data = self
            .nodes
            .get_mut(&owner)
            .ok_or(ExpressionError::NodeNotFound(owner))?;

        debug_assert!(!ns_data.inputs().contains(&data.id()));

        ns_data.inputs_mut().push(data.id());

        self.node_inputs.insert(data.id(), data);

        Ok(id)
    }

    pub(crate) fn remove_node_input(
        &mut self,
        input_id: ExpressionNodeInputId,
    ) -> Result<(), ExpressionError> {
        let ni_data = self
            .node_input(input_id)
            .ok_or(ExpressionError::InputNotFound(input_id))?;
        if ni_data.target().is_some() {
            return Err(ExpressionError::BadInputCleanup(input_id));
        }
        let ns_data = self.nodes.get_mut(&ni_data.owner()).unwrap();
        debug_assert_eq!(
            ns_data.inputs().iter().filter(|x| **x == input_id).count(),
            1
        );
        ns_data.inputs_mut().retain(|x| *x != input_id);
        let prev = self.node_inputs.remove(&input_id);
        debug_assert!(prev.is_some());

        Ok(())
    }

    pub fn add_pure_expression_node<T: 'static + PureExpressionNode>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<PureExpressionNodeHandle<T>, ExpressionError> {
        let id = ExpressionNodeId::new_unique();
        self.nodes.insert(id, ExpressionNodeData::new_empty(id));
        let tools = ExpressionNodeTools::new(id, self);
        let node = Rc::new(PureExpressionNodeWithId::new(
            T::new(tools, args).map_err(|_| ExpressionError::BadNodeInit(id))?,
            id,
        ));
        let node2 = Rc::clone(&node);
        self.nodes.get_mut(&id).unwrap().set_instance(node);
        Ok(PureExpressionNodeHandle::new(node2))
    }

    pub fn add_stateful_expression_node<T: 'static + StatefulExpressionNode>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<StatefulExpressionNodeHandle<T>, ExpressionError> {
        let id = ExpressionNodeId::new_unique();
        self.nodes.insert(id, ExpressionNodeData::new_empty(id));
        let tools = ExpressionNodeTools::new(id, self);
        let node = Rc::new(StatefulExpressionNodeWithId::new(
            T::new(tools, args).map_err(|_| ExpressionError::BadNodeInit(id))?,
            id,
        ));
        let node2 = Rc::clone(&node);
        self.nodes.get_mut(&id).unwrap().set_instance(node);
        Ok(StatefulExpressionNodeHandle::new(node2))
    }

    pub(crate) fn remove_node(&mut self, node_id: ExpressionNodeId) -> Result<(), ExpressionError> {
        if !self.nodes.contains_key(&node_id) {
            return Err(ExpressionError::NodeNotFound(node_id));
        }

        // Does the node still own any inputs?
        if self.node_inputs.values().any(|d| d.owner() == node_id) {
            return Err(ExpressionError::BadNodeCleanup(node_id));
        }
        // Is anything connected to the node?
        if self.destinations(node_id.into()).count() > 0 {
            return Err(ExpressionError::BadNodeCleanup(node_id));
        }

        debug_assert!(self.nodes.get(&node_id).unwrap().inputs().is_empty());

        self.nodes.remove(&node_id);

        Ok(())
    }

    pub(crate) fn connect_node_input(
        &mut self,
        input_id: ExpressionNodeInputId,
        target: ExpressionTarget,
    ) -> Result<(), ExpressionError> {
        match target {
            ExpressionTarget::Node(nsid) => {
                if !self.nodes.contains_key(&nsid) {
                    return Err(ExpressionError::NodeNotFound(nsid));
                }
            }
            ExpressionTarget::Parameter(giid) => {
                if !self.parameters.contains(&giid) {
                    return Err(ExpressionError::ParameterNotFound(giid));
                }
            }
        }
        let data = self
            .node_inputs
            .get_mut(&input_id)
            .ok_or(ExpressionError::InputNotFound(input_id))?;
        if let Some(current_target) = data.target() {
            return Err(ExpressionError::InputOccupied {
                input_id,
                current_target,
            });
        }
        data.set_target(Some(target));

        Ok(())
    }

    pub(crate) fn disconnect_node_input(
        &mut self,
        input_id: ExpressionNodeInputId,
    ) -> Result<(), ExpressionError> {
        let data = self
            .node_inputs
            .get_mut(&input_id)
            .ok_or(ExpressionError::InputNotFound(input_id))?;
        if data.target().is_none() {
            return Err(ExpressionError::InputUnoccupied(input_id));
        }
        data.set_target(None);
        Ok(())
    }

    pub(crate) fn add_parameter(&mut self) -> ExpressionGraphParameterId {
        let id = ExpressionGraphParameterId::new_unique();
        self.parameters.push(id);
        id
    }

    pub(crate) fn remove_parameter(
        &mut self,
        input_id: ExpressionGraphParameterId,
    ) -> Result<(), ExpressionError> {
        if self.parameters.iter().filter(|x| **x == input_id).count() != 1 {
            return Err(ExpressionError::ParameterNotFound(input_id));
        }
        if self
            .node_inputs
            .values()
            .any(|x| x.target() == Some(ExpressionTarget::Parameter(input_id)))
        {
            return Err(ExpressionError::BadParameterCleanup(input_id));
        }
        if self
            .results
            .iter()
            .any(|x| x.target() == Some(ExpressionTarget::Parameter(input_id)))
        {
            return Err(ExpressionError::BadParameterCleanup(input_id));
        }

        self.parameters.retain(|x| *x != input_id);
        Ok(())
    }

    pub(crate) fn add_result(&mut self, default_value: f32) -> ExpressionGraphResultId {
        let id = ExpressionGraphResultId::new_unique();
        let data = ExpressionGraphResultData::new(id, default_value);
        self.results.push(data);
        id
    }

    pub(crate) fn remove_result(
        &mut self,
        output_id: ExpressionGraphResultId,
    ) -> Result<(), ExpressionError> {
        if self.results.iter().filter(|x| x.id() == output_id).count() != 1 {
            return Err(ExpressionError::BadResultCleanup(output_id));
        }
        if self
            .results
            .iter()
            .filter(|x| x.id() == output_id)
            .next()
            .unwrap()
            .target()
            .is_some()
        {
            return Err(ExpressionError::BadResultCleanup(output_id));
        }
        self.results.retain(|x| x.id() != output_id);
        Ok(())
    }

    pub(crate) fn connect_result(
        &mut self,
        output_id: ExpressionGraphResultId,
        target: ExpressionTarget,
    ) -> Result<(), ExpressionError> {
        match target {
            ExpressionTarget::Node(nsid) => {
                if !self.nodes.contains_key(&nsid) {
                    return Err(ExpressionError::NodeNotFound(nsid));
                }
            }
            ExpressionTarget::Parameter(giid) => {
                if !self.parameters.contains(&giid) {
                    return Err(ExpressionError::ParameterNotFound(giid));
                }
            }
        };
        let data = self
            .results
            .iter_mut()
            .filter(|x| x.id() == output_id)
            .next()
            .ok_or(ExpressionError::ResultNotFound(output_id))?;
        if let Some(current_target) = data.target() {
            return Err(ExpressionError::ResultOccupied {
                result_id: output_id,
                current_target,
            });
        }
        data.set_target(Some(target));
        Ok(())
    }

    pub(crate) fn disconnect_result(
        &mut self,
        output_id: ExpressionGraphResultId,
    ) -> Result<(), ExpressionError> {
        let data = self
            .results
            .iter_mut()
            .filter(|x| x.id() == output_id)
            .next()
            .ok_or(ExpressionError::ResultNotFound(output_id))?;
        if data.target().is_none() {
            return Err(ExpressionError::ResultUnoccupied(output_id));
        }
        data.set_target(None);
        Ok(())
    }

    //-------------------------------------------

    /// Create a ExpressionNodeTools instance for making
    /// changes to a single expression node pass them to the
    /// provided closure. The caller is assumed to already have
    /// a handle to the expression node in question.
    pub fn apply_expression_node_tools<F: FnOnce(ExpressionNodeTools)>(
        &mut self,
        node_id: ExpressionNodeId,
        f: F,
    ) -> Result<(), ExpressionError> {
        let tools = ExpressionNodeTools::new(node_id, self);
        f(tools);
        Ok(())
    }

    /// Helper method for editing the expression graph, detecting any errors,
    /// rolling back the changes if any errors were found, and otherwise
    /// keeping the change.
    pub fn try_make_change<R, F: FnOnce(&mut ExpressionGraph) -> Result<R, ExpressionError>>(
        &mut self,
        f: F,
    ) -> Result<R, ExpressionError> {
        if let Err(e) = self.validate() {
            panic!(
                "Called try_make_change() on an ExpressionGraph which is already invalid: {:?}",
                e
            );
        }
        let previous_graph = self.clone();
        let res = f(self);
        if res.is_err() {
            *self = previous_graph;
            return res;
        } else if let Err(e) = self.validate() {
            *self = previous_graph;
            return Err(e);
        }
        res
    }

    pub fn validate(&self) -> Result<(), ExpressionError> {
        match find_expression_error(self) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

pub fn clean_up_and_remove_expression_node(
    graph: &mut ExpressionGraph,
    id: ExpressionNodeId,
) -> Result<(), ExpressionError> {
    let mut inputs_to_remove = Vec::new();
    let mut inputs_to_disconnect = Vec::new();
    let mut results_to_disconnect = Vec::new();

    let node = graph.node(id).ok_or(ExpressionError::NodeNotFound(id))?;

    for ni in node.inputs() {
        inputs_to_remove.push(*ni);
        if graph.node_input(*ni).unwrap().target().is_some() {
            inputs_to_disconnect.push(*ni);
        }
    }

    for ni in graph.node_inputs().values() {
        if ni.target() == Some(ExpressionTarget::Node(id)) {
            inputs_to_disconnect.push(ni.id());
        }
    }

    for go in graph.results() {
        if go.target() == Some(ExpressionTarget::Node(id)) {
            results_to_disconnect.push(go.id());
        }
    }

    // ---

    for ni in inputs_to_disconnect {
        graph.disconnect_node_input(ni)?;
    }

    for go in results_to_disconnect {
        graph.disconnect_result(go)?;
    }

    for ni in inputs_to_remove {
        graph.remove_node_input(ni)?;
    }

    graph.remove_node(id)?;

    Ok(())
}
