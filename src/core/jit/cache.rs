use std::{cell::RefCell, collections::HashMap};

use hashstash::ObjectHash;

use crate::core::{
    expression::expressiongraph::ExpressionGraph,
    sound::{
        expression::{ExpressionParameterMapping, ProcessorExpressionLocation},
        soundgraph::SoundGraph,
    },
};

use super::{
    compiledexpression::{CompiledExpressionArtefact, CompiledExpressionFunction},
    jit::Jit,
};

struct Entry<'ctx> {
    artefact: CompiledExpressionArtefact<'ctx>,
    // TODO: info about how recently the entry was used,
    // in order to help clean things out efficiently.
    // NOTE that things may still be in use on the audio
    // thread. CompiledExpressionArtefact's internal Arc
    // and its reference count may be of use. Alternatively,
    // properties of the current graph may suffice.
}

pub(crate) struct JitCache<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    cache: RefCell<HashMap<(ProcessorExpressionLocation, ObjectHash), Entry<'ctx>>>,
}

impl<'ctx> JitCache<'ctx> {
    pub(crate) fn new(inkwell_context: &'ctx inkwell::context::Context) -> JitCache<'ctx> {
        JitCache {
            inkwell_context,
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn get_compiled_expression(
        &self,
        location: ProcessorExpressionLocation,
        expr_graph: &ExpressionGraph,
        mapping: &ExpressionParameterMapping,
        graph: &SoundGraph,
    ) -> CompiledExpressionFunction<'ctx> {
        let hash = ObjectHash::from_stashable(graph);
        self.cache
            .borrow_mut()
            .entry((location, hash))
            .or_insert_with(|| {
                let jit = Jit::new(self.inkwell_context);
                let artefact = jit.compile_expression(expr_graph, mapping, graph);
                Entry { artefact }
            })
            .artefact
            .make_function()
    }
}
