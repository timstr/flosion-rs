use std::collections::{HashMap, HashSet};

use hashstash::{
    stash_clone_with_context, InplaceUnstasher, ObjectHash, Order, Stash, Stashable, Stasher,
    UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::{
    core::{
        expression::expressiongraphvalidation::find_expression_error,
        stashing::{ExpressionUnstashingContext, StashingContext, UnstashingContext},
        uniqueid::UniqueId,
    },
    ui_core::arguments::ParsedArguments,
};

use super::{
    expressiongrapherror::ExpressionError,
    expressioninput::{ExpressionInput, ExpressionInputId, ExpressionInputLocation},
    expressionnode::{AnyExpressionNode, ExpressionNodeId},
    expressionobject::ExpressionObjectFactory,
};

/// The set of things that an expression node input or graph output
/// can draw from, i.e. which produce a numeric value when evaluated.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum ExpressionTarget {
    /// The result of an expression node in the graph
    Node(ExpressionNodeId),
    /// One of the supplied parameters to the graph
    Parameter(ExpressionGraphParameterId),
}

impl From<ExpressionNodeId> for ExpressionTarget {
    fn from(value: ExpressionNodeId) -> Self {
        ExpressionTarget::Node(value)
    }
}

impl From<ExpressionGraphParameterId> for ExpressionTarget {
    fn from(value: ExpressionGraphParameterId) -> Self {
        ExpressionTarget::Parameter(value)
    }
}

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

    pub(crate) fn results_mut(&mut self) -> &mut [ExpressionInput] {
        &mut self.results
    }

    pub(crate) fn inputs_connected_to<'a>(
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
        expression_object_factory: &ExpressionObjectFactory,
        f: F,
    ) -> Result<R, ExpressionError> {
        if let Err(e) = self.validate() {
            panic!(
                "Called try_make_change() on an ExpressionGraph which is already invalid: {:?}",
                e
            );
        }
        let (previous_graph, stash_handle) = stash_clone_with_context(
            self,
            stash,
            StashingContext::new_stashing_normally(),
            ExpressionUnstashingContext::new(expression_object_factory),
        )
        .unwrap();

        debug_assert_eq!(
            stash_handle.object_hash(),
            ObjectHash::from_stashable_and_context(self, StashingContext::new_stashing_normally()),
            "ExpressionGraph hash differs after stash-cloning"
        );

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

    pub fn pretty_print(&self) -> String {
        "I am an expression graph".to_string()
    }
}

impl Stashable<StashingContext> for ExpressionGraph {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        // nodes
        stasher.array_of_proxy_objects(
            self.nodes.values(),
            |node, stasher| {
                // id (needed during in-place unstashing to find existing nodes)
                stasher.u64(node.id().value() as _);

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

impl<'a> Unstashable<ExpressionUnstashingContext<'a>> for ExpressionGraph {
    fn unstash(
        unstasher: &mut Unstasher<ExpressionUnstashingContext>,
    ) -> Result<ExpressionGraph, UnstashError> {
        let mut graph = ExpressionGraph::new();

        // nodes
        unstasher.array_of_proxy_objects(|unstasher| {
            // id
            let id = ExpressionNodeId::new(unstasher.u64()? as _);

            // type name
            let type_name = unstasher.string()?;

            let mut node = unstasher
                .context()
                .expression_object_factory()
                .create(&type_name, &ParsedArguments::new_empty())
                .into_boxed_expression_node()
                .unwrap();

            // contents
            unstasher.object_proxy_inplace_with_context(
                |unstasher| node.unstash_inplace(unstasher),
                (),
            )?;

            debug_assert_eq!(node.id(), id);

            graph.add_expression_node(node);

            Ok(())
        })?;

        // parameters
        graph.parameters = unstasher
            .array_of_u64_iter()?
            .map(|i| ExpressionGraphParameterId::new(i as _))
            .collect();

        // results
        graph.results = unstasher.array_of_objects_vec_with_context(())?;

        Ok(graph)
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for ExpressionGraph {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        let time_to_write = unstasher.time_to_write();

        let mut nodes_to_keep: HashSet<ExpressionNodeId> = HashSet::new();

        // nodes
        unstasher.array_of_proxy_objects(|unstasher| {
            // id
            let id = ExpressionNodeId::new(unstasher.u64()? as _);

            // type name
            let type_name = unstasher.string()?;

            if let Some(existing_node) = self.node_mut(id) {
                unstasher.object_proxy_inplace_with_context(
                    |unstasher| existing_node.unstash_inplace(unstasher),
                    (),
                )?;
            } else {
                let mut node = unstasher
                    .context()
                    .expression_object_factory()
                    .create(&type_name, &ParsedArguments::new_empty())
                    .into_boxed_expression_node()
                    .unwrap();

                // contents
                unstasher.object_proxy_inplace_with_context(
                    |unstasher| node.unstash_inplace(unstasher),
                    (),
                )?;

                if time_to_write {
                    self.add_expression_node(node);
                }
            }

            nodes_to_keep.insert(id);

            Ok(())
        })?;

        if time_to_write {
            // remove nodes which were not stashed
            self.nodes.retain(|id, _| nodes_to_keep.contains(id));
        }

        // parameters
        let parameters = unstasher.array_of_u64_iter()?;

        if time_to_write {
            self.parameters = parameters
                .map(|i| ExpressionGraphParameterId::new(i as _))
                .collect();
        }

        // results
        unstasher.array_of_objects_vec_inplace_with_context(&mut self.results, ())?;

        Ok(())
    }
}
