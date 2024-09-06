use super::{
    expressiongraph::ExpressionGraph, expressionnode::ExpressionNodeId,
    expressionnodeinput::ExpressionNodeInputHandle,
};

pub struct ExpressionNodeTools<'a> {
    node_id: ExpressionNodeId,
    graph: &'a mut ExpressionGraph,
}

impl<'a> ExpressionNodeTools<'a> {
    pub(crate) fn new(
        node_id: ExpressionNodeId,
        graph: &'a mut ExpressionGraph,
    ) -> ExpressionNodeTools<'a> {
        ExpressionNodeTools { node_id, graph }
    }

    pub fn add_input(&mut self, default_value: f32) -> ExpressionNodeInputHandle {
        let id = self
            .graph
            .add_node_input(self.node_id, default_value)
            .unwrap();
        ExpressionNodeInputHandle::new(id, self.node_id)
    }

    pub fn remove_input(&mut self, handle: ExpressionNodeInputHandle) {
        self.graph.remove_node_input(handle.id()).unwrap();
    }
}
