use std::collections::HashMap;

use crate::core::{
    jit::{cache::JitCache, compiledexpression::CompiledExpressionFunction},
    sound::{
        expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
};

use super::stategraphnode::{
    SharedCompiledProcessor, StateGraphNodeValue, UniqueCompiledSoundProcessor,
};

/// Struct through which compilation of sound graph components for direct
/// execution on the audio thread is performed. SoundGraphCompiler combines
/// both the creation of executable nodes for sound processors and their inputs
/// as well as JIT compilation of expressions.
pub struct SoundGraphCompiler<'a, 'ctx> {
    /// The current sound graph topology
    topology: &'a SoundGraphTopology,

    /// The JIT cache for compiling expressions
    jit_cache: &'a JitCache<'ctx>,

    /// Cache of all nodes for processors which are static and thus should
    /// only have one single, shared state graph node.
    // TODO: when implementing partial state graph edits, make sure this is
    // maintained between topology updates.
    static_processor_nodes: HashMap<SoundProcessorId, SharedCompiledProcessor<'ctx>>,
}

impl<'a, 'ctx> SoundGraphCompiler<'a, 'ctx> {
    /// Create a new SoundGraphCompiler instance. The static processor cache will be
    /// empty, and new nodes will be genereated for static processors
    /// the first time they are encountered by this SoundGraphCompiler instance.
    pub(crate) fn new(
        topology: &'a SoundGraphTopology,
        jit_cache: &'a JitCache<'ctx>,
    ) -> SoundGraphCompiler<'a, 'ctx> {
        SoundGraphCompiler {
            topology,
            jit_cache,
            static_processor_nodes: HashMap::new(),
        }
    }

    /// Compile a sound processor, creating an executable state graph node.
    /// If the processor is static, its node will be cached to ensure that multiple
    /// requests for the same static node receive the same (single) shared node.
    pub(crate) fn compile_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
    ) -> StateGraphNodeValue<'ctx> {
        let proc = self.topology.sound_processor(processor_id).unwrap();
        if proc.instance().is_static() {
            if let Some(node) = self.static_processor_nodes.get(&processor_id) {
                StateGraphNodeValue::Shared(node.clone())
            } else {
                let node = SharedCompiledProcessor::new(proc.instance_arc().compile(self));
                self.static_processor_nodes
                    .insert(processor_id, node.clone());
                StateGraphNodeValue::Shared(node)
            }
        } else {
            // TODO: for shared dynamic processors, some kind of clever
            // book-keeping will be needed here
            StateGraphNodeValue::Unique(UniqueCompiledSoundProcessor::new(
                proc.instance_arc().compile(self),
            ))
        }
    }

    /// Compile an expression using the JIT compiler, or retrieve
    /// it if it's already compiled
    pub(crate) fn get_compiled_expression(
        &self,
        id: SoundExpressionId,
    ) -> CompiledExpressionFunction<'ctx> {
        self.jit_cache.get_compiled_expression(id, self.topology)
    }

    /// Compile a sound input node for execution on the audio thread
    /// as part of the state graph. This is called automatically when
    /// a sound processor node is compiled.
    pub(crate) fn compile_sound_input(
        &mut self,
        sound_input_id: SoundInputId,
    ) -> StateGraphNodeValue<'ctx> {
        let input_data = self.topology.sound_input(sound_input_id).unwrap();
        match input_data.target() {
            Some(spid) => {
                let mut node = self.compile_sound_processor(spid);
                if let StateGraphNodeValue::Shared(shared_node) = &mut node {
                    shared_node
                        .borrow_cache_mut()
                        .add_target_input(sound_input_id);
                }
                node
            }
            None => StateGraphNodeValue::Empty,
        }
    }
}
