use std::{cell::RefCell, collections::HashMap};

use hashstash::ObjectHash;

use crate::core::{
    expression::expressiongraph::ExpressionGraph,
    sound::{
        expression::{ExpressionParameterMapping, ProcessorExpressionLocation},
        soundgraph::SoundGraph,
    },
    stashing::StashingContext,
};

use super::{
    compiledexpression::{CompiledExpressionArtefact, CompiledExpressionFunction},
    jit::{Jit, JitMode},
};

struct Entry<'ctx> {
    artefact: CompiledExpressionArtefact<'ctx>,
    location: ProcessorExpressionLocation,
    // TODO: memory usage tracking. Does LLVM report that in any way?
    // TODO: info about how recently the entry was used,
    // in order to help clean things out efficiently.
    // NOTE that things may still be in use on the audio
    // thread. CompiledExpressionArtefact's internal Arc
    // and its reference count may be of use. Alternatively,
    // properties of the current graph may suffice.
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
struct ExpressionKey {
    hash: ObjectHash,
    mode: JitMode,
}

pub(crate) struct JitCache<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    cache: HashMap<ExpressionKey, Entry<'ctx>>,
    requests: RefCell<Vec<(ProcessorExpressionLocation, ObjectHash, JitMode)>>,
}

impl<'ctx> JitCache<'ctx> {
    pub(crate) fn new(inkwell_context: &'ctx inkwell::context::Context) -> JitCache<'ctx> {
        JitCache {
            inkwell_context,
            cache: HashMap::new(),
            requests: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        // Remove any expressions no longer in the graph.
        self.cache.retain(|_, entry| graph.contains(entry.location));

        // Compile all expressions normally
        for proc_data in graph.sound_processors().values() {
            proc_data.foreach_expression(|expr, location| {
                let expr_hash = Self::hash_expr(expr.graph(), expr.mapping());
                let key = ExpressionKey {
                    hash: expr_hash,
                    mode: JitMode::Normal,
                };
                self.cache.entry(key).or_insert_with(|| {
                    let jit = Jit::new(self.inkwell_context);
                    let artefact = jit.compile_expression(
                        expr.graph(),
                        expr.mapping(),
                        graph,
                        JitMode::Normal,
                    );
                    Entry { artefact, location }
                });
            });
        }

        for (location, req_hash, mode) in self.requests.borrow_mut().drain(..) {
            graph
                .sound_processor(location.processor())
                .unwrap()
                .with_expression(location.expression(), |expr| {
                    let expr_hash = Self::hash_expr(expr.graph(), expr.mapping());
                    if expr_hash != req_hash {
                        return;
                    }
                    let key = ExpressionKey {
                        hash: expr_hash,
                        mode,
                    };
                    self.cache.entry(key).or_insert_with(|| {
                        let jit = Jit::new(self.inkwell_context);
                        let artefact =
                            jit.compile_expression(expr.graph(), expr.mapping(), graph, mode);
                        Entry { artefact, location }
                    });
                })
                .unwrap();
        }
    }

    pub(crate) fn request_compiled_expression(
        &self,
        location: ProcessorExpressionLocation,
        expr_graph: &ExpressionGraph,
        mapping: &ExpressionParameterMapping,
        mode: JitMode,
    ) -> Option<CompiledExpressionFunction<'ctx>> {
        let expr_hash = Self::hash_expr(expr_graph, mapping);
        let key = ExpressionKey {
            hash: expr_hash,
            mode,
        };
        if let Some(f) = self
            .cache
            .get(&key)
            .map(|entry| entry.artefact.make_function())
        {
            Some(f)
        } else {
            self.requests.borrow_mut().push((location, expr_hash, mode));
            None
        }
    }

    fn hash_expr(expr_graph: &ExpressionGraph, mapping: &ExpressionParameterMapping) -> ObjectHash {
        ObjectHash::with_stasher(|stasher| {
            stasher.object_with_context(expr_graph, StashingContext::new_checking_recompilation());
            stasher.object(mapping);
        })
    }
}
