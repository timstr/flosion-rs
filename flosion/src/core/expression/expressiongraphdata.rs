use std::rc::Rc;

use super::{
    expressiongraph::ExpressionGraphParameterId,
    expressionnode::{AnyExpressionNode, ExpressionNodeId},
};

/// An expression node instance and its immediate topological
/// information, intended to be stored in ExpressionGraph.
#[derive(Clone)]
pub(crate) struct ExpressionNodeData {
    id: ExpressionNodeId,
    instance: Option<Rc<dyn AnyExpressionNode>>,
}

impl ExpressionNodeData {
    /// Create a new ExpressionNodeData which does not yet contain an
    /// instance, and is thus only partially valid. Attempts to access
    /// the instance before it is supplied with `set_instance()` will
    /// panic. This enable two-phase initialization, e.g. to make
    /// safe topological changes in the instance's `new()` method.
    pub(super) fn new_empty(id: ExpressionNodeId) -> ExpressionNodeData {
        ExpressionNodeData { id, instance: None }
    }

    /// Get the expression node's id
    pub(crate) fn id(&self) -> ExpressionNodeId {
        self.id
    }

    /// Access the expression node instance
    pub(crate) fn instance(&self) -> &dyn AnyExpressionNode {
        self.instance.as_deref().unwrap()
    }

    /// Access the expression node instance as an Rc
    // TODO: This is probably only used to create a graph object handle. Make that easier.
    pub(crate) fn instance_rc(&self) -> Rc<dyn AnyExpressionNode> {
        Rc::clone(self.instance.as_ref().unwrap())
    }

    /// Set the instance, if self was created with `new_empty()`
    pub(crate) fn set_instance(&mut self, instance: Rc<dyn AnyExpressionNode>) {
        assert!(self.instance.is_none());
        self.instance = Some(instance);
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
