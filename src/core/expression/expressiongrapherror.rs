use super::{
    expressiongraph::{ExpressionGraphParameterId, ExpressionGraphResultId},
    expressiongraphdata::ExpressionTarget,
    expressionnode::ExpressionNodeId,
    expressionnodeinput::ExpressionNodeInputId,
    path::ExpressionPath,
};

/// An error describing how an operation on an expression graph went wrong
#[derive(Debug, Eq, PartialEq)]
pub enum ExpressionError {
    /// The node could not be added because its id collides with another existing node
    NodeIdTaken(ExpressionNodeId),
    /// No node with the given id was found
    NodeNotFound(ExpressionNodeId),
    /// The node was not cleanly initialized when it was added
    BadNodeInit(ExpressionNodeId),
    /// The node was still bound to other parts of the expression graph when it was
    /// attempted to be removed
    BadNodeCleanup(ExpressionNodeId),
    /// The input could not be added because its id collides with another existing input
    InputIdTaken(ExpressionNodeInputId),
    /// No input with the given id was found
    InputNotFound(ExpressionNodeInputId),
    /// The input was not cleanly initialized when it was added
    BadInputInit(ExpressionNodeInputId),
    /// The input was still bound to other parts of the expression graph when it was
    /// attempted to be removed
    BadInputCleanup(ExpressionNodeInputId),
    /// The input couldn't be connected because it's already connected to something else
    InputOccupied {
        input_id: ExpressionNodeInputId,
        current_target: ExpressionTarget,
    },
    /// The input couldn't be disconnected because it's already disconnected
    InputUnoccupied(ExpressionNodeInputId),
    /// The expression graph contains a cycle, which is illegal
    CircularDependency { cycle: ExpressionPath },
    /// The graph parameter could not be added because its id collides with another
    /// existing parameter
    ParameterIdTaken(ExpressionGraphParameterId),
    /// No parameter with the given id was found
    ParameterNotFound(ExpressionGraphParameterId),
    /// The parameter was still bound to other parts of the expression graph when it
    /// was attempted to be removed
    BadParameterCleanup(ExpressionGraphParameterId),
    /// The graph result could not be added because its id collides with another
    /// existing graph result
    ResultIdTaken(ExpressionGraphResultId),
    /// No graph result with the given id was found
    ResultNotFound(ExpressionGraphResultId),
    /// The graph result was not cleanly initialized when it was added
    BadResultInit(ExpressionGraphResultId),
    /// The graph result was still bound to other parts of the expression graph when it
    /// was attempted to be removed
    BadResultCleanup(ExpressionGraphResultId),
    /// The graph result couldn't be connected because it's already connected to
    /// something else
    ResultOccupied {
        result_id: ExpressionGraphResultId,
        current_target: ExpressionTarget,
    },
    /// The graph result couldn't be disconnected because it's already disconnected
    ResultUnoccupied(ExpressionGraphResultId),
}
