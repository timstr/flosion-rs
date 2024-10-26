use super::{
    expressiongraph::ExpressionGraphParameterId, expressioninput::ExpressionInputId,
    expressionnode::ExpressionNodeId,
};

#[derive(Debug, Eq, PartialEq)]
pub enum ExpressionError {
    NodeNotFound(ExpressionNodeId),
    NodeInputNotFound(ExpressionNodeId, ExpressionInputId),
    CircularDependency,
    ParameterNotFound(ExpressionGraphParameterId),
    ResultNotFound(ExpressionInputId),
}
