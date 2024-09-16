use crate::core::uniqueid::UniqueId;

pub struct ExpressionNodeInputTag;

pub type ExpressionNodeInputId = UniqueId<ExpressionNodeInputTag>;

pub struct ExpressionNodeInputHandle {
    id: ExpressionNodeInputId,
}

impl ExpressionNodeInputHandle {
    pub(crate) fn new(id: ExpressionNodeInputId) -> ExpressionNodeInputHandle {
        ExpressionNodeInputHandle { id }
    }

    pub fn id(&self) -> ExpressionNodeInputId {
        self.id
    }
}
