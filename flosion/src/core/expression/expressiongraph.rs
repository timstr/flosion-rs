use std::collections::HashMap;

use hashstash::{
    stash_clone_proxy, stash_clone_proxy_with_context, Order, Stash, StashHandle, Stashable,
    Stasher, UnstashError, Unstasher,
};

use crate::{
    core::{
        expression::expressiongraphvalidation::find_expression_error, stashing::StashingContext,
        uniqueid::UniqueId,
    },
    ui_core::arguments::ParsedArguments,
};

use super::{
    expressiongraphdata::ExpressionTarget,
    expressiongrapherror::ExpressionError,
    expressioninput::{ExpressionInput, ExpressionInputId, ExpressionInputLocation},
    expressionnode::{AnyExpressionNode, ExpressionNodeId},
    expressionobject::ExpressionObjectFactory,
};

pub struct ExpressionGraphParameterTag;

/// A unique integer identifier for an expression graph parameter.
/// No two parameters of the same expression graph may share
/// the same id.
pub type ExpressionGraphParameterId = UniqueId<ExpressionGraphParameterTag>;

pub struct ExpressionGraph {
    nodes: HashMap<ExpressionNodeId, Box<dyn AnyExpressionNode>>,
    parameters: Vec<ExpressionGraphParameterId>,
    results: Vec<ExpressionInput>,
}

impl ExpressionGraph {
    pub(crate) fn new() -> ExpressionGraph {
        ExpressionGraph {
            nodes: HashMap::new(),
            parameters: Vec::new(),
            results: Vec::new(),
        }
    }

    pub(crate) fn node(&self, id: ExpressionNodeId) -> Option<&dyn AnyExpressionNode> {
        match self.nodes.get(&id) {
            Some(n) => Some(&**n),
            None => None,
        }
    }

    pub(crate) fn node_mut(&mut self, id: ExpressionNodeId) -> Option<&mut dyn AnyExpressionNode> {
        match self.nodes.get_mut(&id) {
            Some(n) => Some(&mut **n),
            None => None,
        }
    }

    pub(crate) fn nodes(&self) -> &HashMap<ExpressionNodeId, Box<dyn AnyExpressionNode>> {
        &self.nodes
    }

    pub(crate) fn parameters(&self) -> &[ExpressionGraphParameterId] {
        &self.parameters
    }

    pub(crate) fn result(&self, id: ExpressionInputId) -> Option<&ExpressionInput> {
        self.results.iter().filter(|x| x.id() == id).next()
    }

    pub(crate) fn result_mut(&mut self, id: ExpressionInputId) -> Option<&mut ExpressionInput> {
        self.results.iter_mut().filter(|x| x.id() == id).next()
    }

    pub(crate) fn results(&self) -> &[ExpressionInput] {
        &self.results
    }

    // TODO: rename to e.g. inputs_connected_to
    pub(crate) fn destinations<'a>(
        &'a self,
        target: ExpressionTarget,
    ) -> Vec<ExpressionInputLocation> {
        let mut input_locations = Vec::new();
        for node in self.nodes.values() {
            node.foreach_input(|input, loc| {
                if input.target() == Some(target) {
                    input_locations.push(loc);
                }
            });
        }
        for result in &self.results {
            if result.target() == Some(target) {
                input_locations.push(ExpressionInputLocation::GraphResult(result.id()));
            }
        }
        input_locations
    }

    pub fn add_expression_node(&mut self, node: Box<dyn AnyExpressionNode>) {
        let prev = self.nodes.insert(node.id(), node);
        debug_assert!(prev.is_none());
    }

    fn disconnect_all_inputs_from(&mut self, target: ExpressionTarget) {
        for node in self.nodes.values_mut() {
            node.foreach_input_mut(|input, _| {
                if input.target() == Some(target) {
                    input.set_target(None);
                }
            });
        }

        for result in &mut self.results {
            if result.target() == Some(target) {
                result.set_target(None);
            }
        }
    }

    pub(crate) fn remove_node(&mut self, node_id: ExpressionNodeId) -> Result<(), ExpressionError> {
        self.disconnect_all_inputs_from(ExpressionTarget::Node(node_id));

        self.nodes.remove(&node_id);

        Ok(())
    }

    pub(crate) fn input_target(
        &self,
        input_location: ExpressionInputLocation,
    ) -> Result<Option<ExpressionTarget>, ExpressionError> {
        match input_location {
            ExpressionInputLocation::NodeInput(node_id, input_id) => {
                let node = self
                    .node(node_id)
                    .ok_or(ExpressionError::NodeNotFound(node_id))?;
                node.with_input(input_id, |input| input.target())
                    .ok_or(ExpressionError::NodeInputNotFound(node_id, input_id))
            }
            ExpressionInputLocation::GraphResult(result_id) => {
                let result = self
                    .result(result_id)
                    .ok_or(ExpressionError::ResultNotFound(result_id))?;
                Ok(result.target())
            }
        }
    }

    pub(crate) fn connect_input(
        &mut self,
        input_location: ExpressionInputLocation,
        target: Option<ExpressionTarget>,
    ) -> Result<(), ExpressionError> {
        match input_location {
            ExpressionInputLocation::NodeInput(node_id, input_id) => {
                let node = self
                    .node_mut(node_id)
                    .ok_or(ExpressionError::NodeNotFound(node_id))?;
                node.with_input_mut(input_id, |input| input.set_target(target))
                    .ok_or(ExpressionError::NodeInputNotFound(node_id, input_id))
            }
            ExpressionInputLocation::GraphResult(result_id) => {
                let result = self
                    .result_mut(result_id)
                    .ok_or(ExpressionError::ResultNotFound(result_id))?;
                result.set_target(target);
                Ok(())
            }
        }
    }

    pub(crate) fn add_parameter(&mut self) -> ExpressionGraphParameterId {
        let id = ExpressionGraphParameterId::new_unique();
        self.parameters.push(id);
        id
    }

    pub(crate) fn remove_parameter(
        &mut self,
        param_id: ExpressionGraphParameterId,
    ) -> Result<(), ExpressionError> {
        if !self.parameters.iter().any(|x| *x == param_id) {
            return Err(ExpressionError::ParameterNotFound(param_id));
        }
        let param_target = ExpressionTarget::Parameter(param_id);
        for node in self.nodes.values_mut() {
            node.foreach_input_mut(|input, _| {
                if input.target() == Some(param_target) {
                    input.set_target(None);
                }
            });
        }
        for result in &mut self.results {
            if result.target() == Some(param_target) {
                result.set_target(None);
            }
        }

        self.parameters.retain(|x| *x != param_id);
        Ok(())
    }

    pub(crate) fn add_result(&mut self, default_value: f32) -> ExpressionInputId {
        let result = ExpressionInput::new(default_value);
        let id = result.id();
        self.results.push(result);
        id
    }

    pub(crate) fn remove_result(
        &mut self,
        result_id: ExpressionInputId,
    ) -> Result<(), ExpressionError> {
        self.results.retain(|x| x.id() != result_id);
        Ok(())
    }

    // TODO: remove, merge with connect_input
    pub(crate) fn connect_result(
        &mut self,
        result_id: ExpressionInputId,
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
            .filter(|x| x.id() == result_id)
            .next()
            .ok_or(ExpressionError::ResultNotFound(result_id))?;
        data.set_target(Some(target));
        Ok(())
    }

    // TODO: remove
    pub(crate) fn disconnect_result(
        &mut self,
        result_id: ExpressionInputId,
    ) -> Result<(), ExpressionError> {
        let data = self
            .results
            .iter_mut()
            .filter(|x| x.id() == result_id)
            .next()
            .ok_or(ExpressionError::ResultNotFound(result_id))?;
        data.set_target(None);
        Ok(())
    }

    //-------------------------------------------

    /// Helper method for editing the expression graph, detecting any errors,
    /// rolling back the changes if any errors were found, and otherwise
    /// keeping the change.
    pub fn try_make_change<R, F: FnOnce(&mut ExpressionGraph) -> Result<R, ExpressionError>>(
        &mut self,
        stash: &Stash,
        factory: &ExpressionObjectFactory,
        f: F,
    ) -> Result<R, ExpressionError> {
        if let Err(e) = self.validate() {
            panic!(
                "Called try_make_change() on an ExpressionGraph which is already invalid: {:?}",
                e
            );
        }
        let (previous_graph, _) = self.stash_clone(stash, factory).unwrap();
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

impl Stashable for ExpressionGraph {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        // nodes
        stasher.array_of_proxy_objects(
            self.nodes.values(),
            |node, stasher| {
                // type name (needed for factory during unstashing)
                stasher.string(node.as_graph_object().get_dynamic_type().name());

                // contents
                stasher.object_proxy(|stasher| node.stash(stasher));
            },
            Order::Unordered,
        );

        // parameters
        stasher.array_of_u64_iter(self.parameters.iter().map(|i| i.value() as u64));

        // results
        stasher.array_of_objects_slice(&self.results, Order::Ordered);
    }
}

// TODO: extend HashStash to allow some additional parameters during unstashing
// so that its unstashing traits doen't need to be subverted like this
impl ExpressionGraph {
    pub(crate) fn unstash(
        unstasher: &mut Unstasher,
        factory: &ExpressionObjectFactory,
    ) -> Result<ExpressionGraph, UnstashError> {
        let mut graph = ExpressionGraph::new();

        // nodes
        unstasher.array_of_proxy_objects(|unstasher| {
            // type name
            let type_name = unstasher.string()?;

            let mut node = factory
                .create(&type_name, &ParsedArguments::new_empty())
                .into_boxed_expression_node()
                .unwrap();

            // contents
            unstasher.object_proxy_inplace(|unstasher| node.unstash_inplace(unstasher))?;

            graph.add_expression_node(node);

            Ok(())
        })?;

        // parameters
        graph.parameters = unstasher
            .array_of_u64_iter()?
            .map(|i| ExpressionGraphParameterId::new(i as _))
            .collect();

        // results
        graph.results = unstasher.array_of_objects_vec()?;

        Ok(graph)
    }

    pub(crate) fn stash_clone(
        &self,
        stash: &Stash,
        factory: &ExpressionObjectFactory,
    ) -> Result<(ExpressionGraph, StashHandle<ExpressionGraph>), UnstashError> {
        stash_clone_proxy_with_context(
            self,
            stash,
            |unstasher| ExpressionGraph::unstash(unstasher, factory),
            &StashingContext::new_stashing_normally(),
        )
    }
}
