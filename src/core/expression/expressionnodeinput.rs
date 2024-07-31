use crate::core::uniqueid::UniqueId;

use super::expressionnode::ExpressionNodeId;

pub struct ExpressionNodeInputTag;

pub type ExpressionNodeInputId = UniqueId<ExpressionNodeInputTag>;

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
