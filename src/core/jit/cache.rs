use std::collections::HashMap;

use crate::core::sound::{expression::ProcessorExpressionLocation, soundgraph::SoundGraph};

use super::{
    compiledexpression::{CompiledExpressionArtefact, CompiledExpressionFunction},
    jit::Jit,
};

struct Entry<'ctx> {
    artefact: CompiledExpressionArtefact<'ctx>,
    // TODO: memory usage tracking. Does LLVM report that in any way?
    // TODO: info about how recently the entry was used,
    // in order to help clean things out efficiently.
    // NOTE that things may still be in use on the audio
    // thread. CompiledExpressionArtefact's internal Arc
    // and its reference count may be of use. Alternatively,
    // properties of the current graph may suffice.
}

pub(crate) struct JitCache<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    cache: HashMap<ProcessorExpressionLocation, Entry<'ctx>>,
}

impl<'ctx> JitCache<'ctx> {
    pub(crate) fn new(inkwell_context: &'ctx inkwell::context::Context) -> JitCache<'ctx> {
        JitCache {
            inkwell_context,
            cache: HashMap::new(),
        }
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        // TODO: hash by individual expression graphs and the
        // handful of arguments they depend on, NOT the whole
        // sound graph, so that unrelated edits don't cause
        // everything to recompile.
        self.cache.clear(); // RIP performance

        for proc_data in graph.sound_processors().values() {
            proc_data.foreach_expression(|expr, location| {
                let jit = Jit::new(self.inkwell_context);
                let artefact = jit.compile_expression(expr.graph(), expr.mapping(), graph);

                self.cache.insert(location, Entry { artefact });
            });
        }
    }

    pub(crate) fn get_compiled_expression(
        &self,
        location: ProcessorExpressionLocation,
    ) -> Option<CompiledExpressionFunction<'ctx>> {
        self.cache
            .get(&location)
            .map(|entry| entry.artefact.make_function())
    }
}
