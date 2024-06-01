use std::collections::HashMap;

use crate::core::{
    jit::{compiledexpression::CompiledExpressionFunction, server::JitServer},
    sound::{
        expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
};

use super::stategraphnode::{NodeTargetValue, SharedProcessorNode, UniqueProcessorNode};

pub struct NodeGen<'a, 'ctx> {
    topology: &'a SoundGraphTopology,
    jit_server: &'a JitServer<'ctx>,
    static_processor_nodes: HashMap<SoundProcessorId, SharedProcessorNode<'ctx>>,
}

impl<'a, 'ctx> NodeGen<'a, 'ctx> {
    pub(crate) fn new(
        topology: &'a SoundGraphTopology,
        jit_server: &'a JitServer<'ctx>,
    ) -> NodeGen<'a, 'ctx> {
        NodeGen {
            topology,
            jit_server,
            static_processor_nodes: HashMap::new(),
        }
    }

    pub(crate) fn compile_processor_node(
        &mut self,
        processor_id: SoundProcessorId,
    ) -> NodeTargetValue<'ctx> {
        let proc = self.topology.sound_processor(processor_id).unwrap();
        if proc.instance().is_static() {
            if let Some(node) = self.static_processor_nodes.get(&processor_id) {
                NodeTargetValue::Shared(node.clone())
            } else {
                let node = SharedProcessorNode::new(proc.instance_arc().make_node(self));
                self.static_processor_nodes
                    .insert(processor_id, node.clone());
                NodeTargetValue::Shared(node)
            }
        } else {
            // TODO: for shared dynamic processors, some kind of clever
            // book-keeping will be needed here
            NodeTargetValue::Unique(UniqueProcessorNode::new(
                proc.instance_arc().make_node(self),
            ))
        }
    }

    pub(crate) fn get_compiled_expression(
        &self,
        id: SoundExpressionId,
    ) -> CompiledExpressionFunction<'ctx> {
        self.jit_server.get_compiled_expression(id, self.topology)
    }

    pub(crate) fn allocate_sound_input_node(
        &mut self,
        sound_input_id: SoundInputId,
    ) -> NodeTargetValue<'ctx> {
        let input_data = self.topology.sound_input(sound_input_id).unwrap();
        match input_data.target() {
            Some(spid) => {
                let mut node = self.compile_processor_node(spid);
                if let NodeTargetValue::Shared(shared_node) = &mut node {
                    shared_node
                        .borrow_data_mut()
                        .add_target_input(sound_input_id);
                }
                node
            }
            None => NodeTargetValue::Empty,
        }
    }
}
