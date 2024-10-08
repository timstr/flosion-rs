use std::rc::Rc;

use super::{
    expressiongraph::{ExpressionGraphParameterId, ExpressionGraphResultId},
    expressionnode::{ExpressionNode, ExpressionNodeId},
    expressionnodeinput::ExpressionNodeInputId,
};

/// An expression node instance and its immediate topological
/// information, intended to be stored in ExpressionGraph.
#[derive(Clone)]
pub(crate) struct ExpressionNodeData {
    id: ExpressionNodeId,
    instance: Option<Rc<dyn ExpressionNode>>,
    inputs: Vec<ExpressionNodeInputId>,
}

impl ExpressionNodeData {
    /// Create a new ExpressionNodeData which does not yet contain an
    /// instance, and is thus only partially valid. Attempts to access
    /// the instance before it is supplied with `set_instance()` will
    /// panic. This enable two-phase initialization, e.g. to make
    /// safe topological changes in the instance's `new()` method.
    pub(super) fn new_empty(id: ExpressionNodeId) -> ExpressionNodeData {
        ExpressionNodeData {
            id,
            instance: None,
            inputs: Vec::new(),
        }
    }

    /// Get the expression node's id
    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.id
    }

    /// Access the expression node instance
    pub(crate) fn instance(&self) -> &dyn ExpressionNode {
        self.instance.as_deref().unwrap()
    }

    /// Access the expression node instance as an Rc
    // TODO: This is probably only used to create a graph object handle. Make that easier.
    pub(crate) fn instance_rc(&self) -> Rc<dyn ExpressionNode> {
        Rc::clone(self.instance.as_ref().unwrap())
    }

    /// Set the instance, if self was created with `new_empty()`
    pub(crate) fn set_instance(&mut self, instance: Rc<dyn ExpressionNode>) {
        assert!(self.instance.is_none());
        self.instance = Some(instance);
    }

    /// Access the list of input ids belonging to the expression node
    pub fn inputs(&self) -> &[ExpressionNodeInputId] {
        &self.inputs
    }

    /// Mutably access the list of input ids belonging to the expression node
    pub fn inputs_mut(&mut self) -> &mut Vec<ExpressionNodeInputId> {
        &mut self.inputs
    }
}

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

/// The set of things that a numeric value from either an
/// expression node or a graph parameter can be sent to,
/// e.g. things that require a numeric value in order to
/// be evaluated.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum ExpressionDestination {
    NodeInput(ExpressionNodeInputId),
    Result(ExpressionGraphResultId),
}

/// The immediate topological data associated with the input of an
/// expression node. Intended to be stored in ExpressionGraph.
#[derive(Clone)]
pub(crate) struct ExpressionNodeInputData {
    id: ExpressionNodeInputId,
    target: Option<ExpressionTarget>,
    owner: ExpressionNodeId,
    default_value: f32,
}

impl ExpressionNodeInputData {
    /// Create a new ExpressionNodeInputData instance
    pub(super) fn new(
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

    /// Get the input's id
    pub fn id(&self) -> ExpressionNodeInputId {
        self.id
    }

    /// Get the input's target
    pub fn target(&self) -> Option<ExpressionTarget> {
        self.target
    }

    /// Set the input's target
    pub fn set_target(&mut self, target: Option<ExpressionTarget>) {
        self.target = target;
    }

    /// Get the input's owner, i.e. the expression node that it belongs to
    pub fn owner(&self) -> ExpressionNodeId {
        self.owner
    }

    /// Get the input's default value, i.e. the value it produces when not
    /// connected to anything
    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}

/// The immediate topological data associated with one of the results of
/// the expression graph.
#[derive(Clone)]
pub(crate) struct ExpressionGraphResultData {
    id: ExpressionGraphResultId,
    target: Option<ExpressionTarget>,
    default_value: f32,
}

impl ExpressionGraphResultData {
    /// Create a new ExpressionGraphResultData instance.
    pub(super) fn new(
        id: ExpressionGraphResultId,
        default_value: f32,
    ) -> ExpressionGraphResultData {
        ExpressionGraphResultData {
            id,
            target: None,
            default_value,
        }
    }

    /// Get the graph result's id
    pub fn id(&self) -> ExpressionGraphResultId {
        self.id
    }

    /// Get the graph result's target
    pub fn target(&self) -> Option<ExpressionTarget> {
        self.target
    }

    /// Set the graph result's target
    pub fn set_target(&mut self, target: Option<ExpressionTarget>) {
        self.target = target;
    }

    /// Get the result's default value, i.e. the value it produces when not
    /// connected to anything
    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}
