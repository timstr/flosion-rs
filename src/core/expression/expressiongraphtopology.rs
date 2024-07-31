use std::hash::Hasher;

use crate::core::revision::revision::{Revisable, Revised, RevisedHashMap, RevisionHash};

use super::{
    expressiongraph::{ExpressionGraphParameterId, ExpressionGraphResultId},
    expressiongraphdata::{
        ExpressionDestination, ExpressionGraphResultData, ExpressionNodeData,
        ExpressionNodeInputData, ExpressionTarget,
    },
    expressiongrapherror::ExpressionError,
    expressionnode::ExpressionNodeId,
    expressionnodeinput::ExpressionNodeInputId,
};

#[derive(Clone)]
pub(crate) struct ExpressionGraphTopology {
    nodes: RevisedHashMap<ExpressionNodeId, ExpressionNodeData>,
    node_inputs: RevisedHashMap<ExpressionNodeInputId, ExpressionNodeInputData>,
    parameters: Vec<ExpressionGraphParameterId>,
    results: Vec<ExpressionGraphResultData>,
}

impl ExpressionGraphTopology {
    pub(crate) fn new() -> ExpressionGraphTopology {
        ExpressionGraphTopology {
            nodes: RevisedHashMap::new(),
            node_inputs: RevisedHashMap::new(),
            parameters: Vec::new(),
            results: Vec::new(),
        }
    }

    pub(crate) fn node_input(
        &self,
        id: ExpressionNodeInputId,
    ) -> Option<&Revised<ExpressionNodeInputData>> {
        self.node_inputs.get(&id)
    }

    pub(crate) fn node(&self, id: ExpressionNodeId) -> Option<&Revised<ExpressionNodeData>> {
        self.nodes.get(&id)
    }

    pub(super) fn node_mut(
        &mut self,
        id: ExpressionNodeId,
    ) -> Option<&mut Revised<ExpressionNodeData>> {
        self.nodes.get_mut(&id)
    }

    pub(crate) fn node_inputs(
        &self,
    ) -> &RevisedHashMap<ExpressionNodeInputId, ExpressionNodeInputData> {
        &self.node_inputs
    }

    pub(crate) fn nodes(&self) -> &RevisedHashMap<ExpressionNodeId, ExpressionNodeData> {
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

    pub fn add_node_input(&mut self, data: ExpressionNodeInputData) -> Result<(), ExpressionError> {
        if data.target().is_some() {
            return Err(ExpressionError::BadInputInit(data.id()));
        }

        if self.node_inputs.contains_key(&data.id()) {
            return Err(ExpressionError::InputIdTaken(data.id()));
        }

        let owner = data.owner();

        let ns_data = self
            .nodes
            .get_mut(&owner)
            .ok_or(ExpressionError::NodeNotFound(owner))?;

        debug_assert!(!ns_data.inputs().contains(&data.id()));

        ns_data.inputs_mut().push(data.id());

        self.node_inputs.insert(data.id(), Revised::new(data));

        Ok(())
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

    pub(crate) fn add_node(&mut self, data: ExpressionNodeData) -> Result<(), ExpressionError> {
        if !data.inputs().is_empty() {
            return Err(ExpressionError::BadNodeInit(data.id()));
        }
        if self.nodes.contains_key(&data.id()) {
            return Err(ExpressionError::NodeIdTaken(data.id()));
        }
        self.nodes.insert(data.id(), Revised::new(data));

        Ok(())
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

    pub(crate) fn add_parameter(
        &mut self,
        input_id: ExpressionGraphParameterId,
    ) -> Result<(), ExpressionError> {
        if self.parameters.contains(&input_id) {
            return Err(ExpressionError::ParameterIdTaken(input_id));
        }
        self.parameters.push(input_id);
        Ok(())
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

    pub(crate) fn add_result(
        &mut self,
        data: ExpressionGraphResultData,
    ) -> Result<(), ExpressionError> {
        if data.target().is_some() {
            return Err(ExpressionError::BadResultInit(data.id()));
        }

        if self.results.iter().filter(|x| x.id() == data.id()).count() > 0 {
            return Err(ExpressionError::ResultIdTaken(data.id()));
        }
        self.results.push(data);
        Ok(())
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
}

impl Revisable for ExpressionGraphTopology {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u64(self.nodes.get_revision().value());
        hasher.write_u64(self.node_inputs.get_revision().value());
        hasher.write_u64(self.parameters.get_revision().value());
        hasher.write_u64(self.results.get_revision().value());
        RevisionHash::new(hasher.finish())
    }
}
