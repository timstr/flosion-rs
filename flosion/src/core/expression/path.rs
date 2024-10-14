use super::{expressionnode::ExpressionNodeId, expressionnodeinput::ExpressionNodeInputId};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExpressionPath {
    pub connections: Vec<(ExpressionNodeId, ExpressionNodeInputId)>,
}

impl ExpressionPath {
    pub fn new(connections: Vec<(ExpressionNodeId, ExpressionNodeInputId)>) -> ExpressionPath {
        ExpressionPath { connections }
    }

    pub fn contains_node(&self, node_id: ExpressionNodeId) -> bool {
        return self
            .connections
            .iter()
            .find(|(nsid, _)| *nsid == node_id)
            .is_some();
    }

    pub fn contains_input(&self, input_id: ExpressionNodeInputId) -> bool {
        return self
            .connections
            .iter()
            .find(|(_, niid)| *niid == input_id)
            .is_some();
    }

    pub fn trim_until_input(&self, input_id: ExpressionNodeInputId) -> ExpressionPath {
        let idx = self
            .connections
            .iter()
            .position(|(_, siid)| *siid == input_id)
            .unwrap();
        let p: Vec<_> = self.connections[idx..].iter().cloned().collect();
        ExpressionPath { connections: p }
    }

    pub fn push(&mut self, node_id: ExpressionNodeId, input_id: ExpressionNodeInputId) {
        self.connections.push((node_id, input_id));
    }

    pub fn pop(&mut self) -> Option<(ExpressionNodeId, ExpressionNodeInputId)> {
        self.connections.pop()
    }
}
