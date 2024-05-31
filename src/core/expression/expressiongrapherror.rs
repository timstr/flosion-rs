use super::{
    expressiongraph::{ExpressionGraphParameterId, ExpressionGraphResultId},
    expressiongraphdata::ExpressionTarget,
    expressionnode::ExpressionNodeId,
    expressionnodeinput::ExpressionNodeInputId,
    path::ExpressionPath,
};

#[derive(Debug, Eq, PartialEq)]
pub enum ExpressionError {
    NodeIdTaken(ExpressionNodeId),
    NodeNotFound(ExpressionNodeId),
    BadNodeInit(ExpressionNodeId),
    BadNodeCleanup(ExpressionNodeId),
    InputIdTaken(ExpressionNodeInputId),
    InputNotFound(ExpressionNodeInputId),
    BadInputInit(ExpressionNodeInputId),
    BadInputCleanup(ExpressionNodeInputId),
    InputOccupied {
        input_id: ExpressionNodeInputId,
        current_target: ExpressionTarget,
    },
    InputUnoccupied(ExpressionNodeInputId),
    CircularDependency {
        cycle: ExpressionPath,
    },
    ParameterIdTaken(ExpressionGraphParameterId),
    ParameterNotFound(ExpressionGraphParameterId),
    BadParameterCleanup(ExpressionGraphParameterId),
    ResultIdTaken(ExpressionGraphResultId),
    ResultNotFound(ExpressionGraphResultId),
    BadResultInit(ExpressionGraphResultId),
    BadResultCleanup(ExpressionGraphResultId),
    ResultOccupied {
        result_id: ExpressionGraphResultId,
        current_target: ExpressionTarget,
    },
    ResultUnoccupied(ExpressionGraphResultId),
}
