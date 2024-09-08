use std::{cell::RefCell, collections::HashMap};

use hashrevise::{Revisable, RevisionHash};

use crate::core::sound::{expression::SoundExpressionId, soundgraph::SoundGraph};

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
    // properties of the current topology may suffice.
}

pub(crate) struct JitCache<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    cache: RefCell<HashMap<(SoundExpressionId, RevisionHash), Entry<'ctx>>>,
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
        id: SoundExpressionId,
        topology: &SoundGraph,
    ) -> CompiledExpressionFunction<'ctx> {
        let revision = topology.get_revision();
        self.cache
            .borrow_mut()
            .entry((id, revision))
            .or_insert_with(|| {
                let jit = Jit::new(self.inkwell_context);
                let artefact = jit.compile_expression(id, topology);
                Entry { artefact }
            })
            .artefact
            .make_function()
    }
}
