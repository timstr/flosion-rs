use std::collections::HashMap;

use crate::core::{
    jit::{compiledexpression::CompiledExpressionFunction, server::JitServer},
    sound::{
        expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
};

use super::stategraphnode::{NodeTargetValue, SharedProcessorNode, UniqueProcessorNode};

/// Struct through which compilation of sound graph components for direct
/// execution on the audio thread is performed. NodeGen combines both the
/// creation of executable nodes for sound processors and their inputs as
/// well as JIT compilation of expressions.
pub struct NodeGen<'a, 'ctx> {
    /// The current sound graph topology
    topology: &'a SoundGraphTopology,

    /// The JIT server for compiling expressions
    jit_server: &'a JitServer<'ctx>,

    /// Cache of all nodes for processors which are static and thus should
    /// only have one single, shared state graph node.
    // TODO: when implementing partial state graph edits, make sure this is
    // maintained between topology updates.
    static_processor_nodes: HashMap<SoundProcessorId, SharedProcessorNode<'ctx>>,
}

impl<'a, 'ctx> NodeGen<'a, 'ctx> {
    /// Create a new NodeGen instance. The static processor cache will be
    /// empty, and new nodes will be genereated for static processors
    /// the first time they are encountered by this NodeGen instance.
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

    /// Compile a sound processor node, creating an executable node value
    /// for the state graph. If the node is static, it will be cached to
    /// to ensure that multiple requests for the same static node receive
    /// the same (single) shared node.
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

    /// Compile an expression using the JIT compiler, or retrieve
    /// it if it's already compiled
    pub(crate) fn get_compiled_expression(
        &self,
        id: SoundExpressionId,
    ) -> CompiledExpressionFunction<'ctx> {
        self.jit_server.get_compiled_expression(id, self.topology)
    }

    /// Compile a sound input node for execution on the audio thread
    /// as part of the state graph. This is called automatically when
    /// a sound processor node is compiled.
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
