use super::{
    expressiongraph::ExpressionGraphIdGenerators, expressiongraphdata::ExpressionNodeInputData,
    expressiongraphtopology::ExpressionGraphTopology, expressionnode::ExpressionNodeId,
    expressionnodeinput::ExpressionNodeInputHandle,
};

pub struct ExpressionNodeTools<'a> {
    node_id: ExpressionNodeId,
    topology: &'a mut ExpressionGraphTopology,
    id_generators: &'a mut ExpressionGraphIdGenerators,
}

impl<'a> ExpressionNodeTools<'a> {
    pub(crate) fn new(
        node_id: ExpressionNodeId,
        topology: &'a mut ExpressionGraphTopology,
        id_generators: &'a mut ExpressionGraphIdGenerators,
    ) -> ExpressionNodeTools<'a> {
        ExpressionNodeTools {
            node_id,
            topology,
            id_generators,
        }
    }

    pub fn add_input(&mut self, default_value: f32) -> ExpressionNodeInputHandle {
        let id = self.id_generators.node_input.next_id();
        let owner = self.node_id;
        let data = ExpressionNodeInputData::new(id, owner, default_value);
        self.topology.add_node_input(data).unwrap();
        ExpressionNodeInputHandle::new(id, owner)
    }

    pub fn remove_input(&mut self, handle: ExpressionNodeInputHandle) {
        self.topology.remove_node_input(handle.id()).unwrap();
    }
}
