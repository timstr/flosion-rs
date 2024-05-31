use crate::core::uniqueid::UniqueId;

use super::expressionnode::ExpressionNodeId;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ExpressionNodeInputId(usize);

impl Default for ExpressionNodeInputId {
    fn default() -> ExpressionNodeInputId {
        ExpressionNodeInputId(1)
    }
}

impl UniqueId for ExpressionNodeInputId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> ExpressionNodeInputId {
        ExpressionNodeInputId(self.0 + 1)
    }
}

pub struct ExpressionNodeInputHandle {
    id: ExpressionNodeInputId,
    owner: ExpressionNodeId,
}

impl ExpressionNodeInputHandle {
    pub(crate) fn new(
        id: ExpressionNodeInputId,
        owner: ExpressionNodeId,
    ) -> ExpressionNodeInputHandle {
        ExpressionNodeInputHandle { id, owner }
    }

    pub fn id(&self) -> ExpressionNodeInputId {
        self.id
    }

    pub(super) fn owner(&self) -> ExpressionNodeId {
        self.owner
    }
}
