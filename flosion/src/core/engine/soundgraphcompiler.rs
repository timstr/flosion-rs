use std::collections::HashMap;

use crate::core::{
    jit::{cache::JitCache, compiledexpression::CompiledExpressionFunction, jit::JitMode},
    sound::{
        expression::ProcessorExpressionLocation, soundgraph::SoundGraph,
        soundprocessor::SoundProcessorId,
    },
};

use super::stategraphnode::{
    SharedCompiledProcessor, StateGraphNodeValue, UniqueCompiledProcessor,
};

/// Struct through which compilation of sound graph components for direct
/// execution on the audio thread is performed. SoundGraphCompiler combines
/// both the creation of executable nodes for sound processors and their inputs
/// as well as JIT compilation of expressions.
pub struct SoundGraphCompiler<'a, 'ctx> {
    /// The current sound graph
    graph: &'a SoundGraph,

    /// The JIT cache for compiling expressions
    jit_cache: &'a JitCache<'ctx>,

    /// Cache of all nodes for processors which are static and thus should
    /// only have one single, shared state graph node.
    // TODO: when implementing partial state graph edits, make sure this is
    // maintained between graph updates.
    static_processor_nodes: HashMap<SoundProcessorId, SharedCompiledProcessor<'ctx>>,
}

impl<'a, 'ctx> SoundGraphCompiler<'a, 'ctx> {
    /// Create a new SoundGraphCompiler instance. The static processor cache will be
    /// empty, and new nodes will be genereated for static processors
    /// the first time they are encountered by this SoundGraphCompiler instance.
    pub(crate) fn new(
        graph: &'a SoundGraph,
        jit_cache: &'a JitCache<'ctx>,
    ) -> SoundGraphCompiler<'a, 'ctx> {
        SoundGraphCompiler {
            graph,
            jit_cache,
            static_processor_nodes: HashMap::new(),
        }
    }

    /// Compile a sound processor, creating an executable state graph node.
    /// If the processor is static, its node will be cached to ensure that multiple
    /// requests for the same static node receive the same (single) shared node.
    pub(crate) fn compile_sound_processor(
        &mut self,
        target: Option<SoundProcessorId>,
    ) -> StateGraphNodeValue<'ctx> {
        let Some(processor_id) = target else {
            return StateGraphNodeValue::Empty;
        };
        let proc = self.graph.sound_processor(processor_id).unwrap();
        if proc.is_static() {
            if let Some(node) = self.static_processor_nodes.get(&processor_id) {
                StateGraphNodeValue::Shared(node.clone())
            } else {
                let node = SharedCompiledProcessor::new(proc.compile(self));
                self.static_processor_nodes
                    .insert(processor_id, node.clone());
                StateGraphNodeValue::Shared(node)
            }
        } else {
            // TODO: for shared dynamic processors, some kind of clever
            // book-keeping will be needed here
            StateGraphNodeValue::Unique(UniqueCompiledProcessor::new(proc.compile(self)))
        }
    }

    pub(crate) fn get_compiled_expression(
        &self,
        location: ProcessorExpressionLocation,
    ) -> Option<CompiledExpressionFunction<'ctx>> {
        self.graph
            .sound_processor(location.processor())
            .unwrap()
            .with_expression(location.expression(), |expr| {
                self.jit_cache.request_compiled_expression(
                    location,
                    expr.graph(),
                    expr.mapping(),
                    JitMode::Normal,
                )
            })
            .unwrap()
    }
}
