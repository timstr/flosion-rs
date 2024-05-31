use std::{hash::Hasher, sync::Arc};

use crate::core::{
    revision::revision::{Revision, RevisionNumber},
    uniqueid::UniqueId,
};

use super::{
    expressiongraph::{ExpressionGraphParameterId, ExpressionGraphResultId},
    expressionnode::{ExpressionNode, ExpressionNodeId},
    expressionnodeinput::ExpressionNodeInputId,
};

#[derive(Clone)]
pub(crate) struct ExpressionNodeData {
    id: ExpressionNodeId,
    instance: Option<Arc<dyn ExpressionNode>>,
    inputs: Vec<ExpressionNodeInputId>,
}

impl ExpressionNodeData {
    pub(crate) fn new(
        id: ExpressionNodeId,
        instance: Arc<dyn ExpressionNode>,
    ) -> ExpressionNodeData {
        ExpressionNodeData {
            id,
            instance: Some(instance),
            inputs: Vec::new(),
        }
    }

    pub(crate) fn new_empty(id: ExpressionNodeId) -> ExpressionNodeData {
        ExpressionNodeData {
            id,
            instance: None,
            inputs: Vec::new(),
        }
    }

    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn ExpressionNode {
        self.instance.as_deref().unwrap()
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn ExpressionNode> {
        Arc::clone(self.instance.as_ref().unwrap())
    }

    pub(crate) fn set_instance(&mut self, instance: Arc<dyn ExpressionNode>) {
        assert!(self.instance.is_none());
        self.instance = Some(instance);
    }

    pub fn inputs(&self) -> &[ExpressionNodeInputId] {
        &self.inputs
    }

    pub fn inputs_mut(&mut self) -> &mut Vec<ExpressionNodeInputId> {
        &mut self.inputs
    }
}

impl Revision for ExpressionNodeData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hasher.write_usize(self.inputs.len());
        for niid in &self.inputs {
            hasher.write_usize(niid.value());
        }
        RevisionNumber::new(hasher.finish())
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum ExpressionTarget {
    // TODO: Empty
    Node(ExpressionNodeId),
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

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum ExpressionDestination {
    NodeInput(ExpressionNodeInputId),
    Result(ExpressionGraphResultId),
}

#[derive(Clone)]
pub(crate) struct ExpressionNodeInputData {
    id: ExpressionNodeInputId,
    target: Option<ExpressionTarget>,
    owner: ExpressionNodeId,
    default_value: f32,
}

impl ExpressionNodeInputData {
    pub(crate) fn new(
        id: ExpressionNodeInputId,
        owner: ExpressionNodeId,
        default_value: f32,
    ) -> ExpressionNodeInputData {
        ExpressionNodeInputData {
            id,
            target: None,
            owner,
            default_value,
        }
    }

    pub fn id(&self) -> ExpressionNodeInputId {
        self.id
    }

    pub fn target(&self) -> Option<ExpressionTarget> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<ExpressionTarget>) {
        self.target = target;
    }

    pub fn owner(&self) -> ExpressionNodeId {
        self.owner
    }

    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}

fn hash_optional_target(target: Option<ExpressionTarget>, hasher: &mut seahash::SeaHasher) {
    match target {
        Some(ExpressionTarget::Parameter(giid)) => {
            hasher.write_u8(1);
            hasher.write_usize(giid.value());
        }
        Some(ExpressionTarget::Node(nsid)) => {
            hasher.write_u8(2);
            hasher.write_usize(nsid.value());
        }
        None => {
            hasher.write_u8(3);
        }
    }
}

impl Revision for ExpressionNodeInputData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hash_optional_target(self.target, &mut hasher);
        hasher.write_usize(self.owner.value());
        hasher.write_u32(self.default_value.to_bits());
        RevisionNumber::new(hasher.finish())
    }
}

#[derive(Clone)]
pub(crate) struct ExpressionGraphResultData {
    id: ExpressionGraphResultId,
    target: Option<ExpressionTarget>,
    default_value: f32,
}

impl ExpressionGraphResultData {
    pub(crate) fn new(
        id: ExpressionGraphResultId,
        default_value: f32,
    ) -> ExpressionGraphResultData {
        ExpressionGraphResultData {
            id,
            target: None,
            default_value,
        }
    }

    pub fn id(&self) -> ExpressionGraphResultId {
        self.id
    }

    pub fn target(&self) -> Option<ExpressionTarget> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<ExpressionTarget>) {
        self.target = target;
    }

    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}

impl Revision for ExpressionGraphResultData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hash_optional_target(self.target, &mut hasher);
        hasher.write_u32(self.default_value.to_bits());
        RevisionNumber::new(hasher.finish())
    }
}
