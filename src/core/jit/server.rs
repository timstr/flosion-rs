use std::collections::HashMap;

use crate::core::{
    revision::revision::RevisionNumber,
    sound::{soundgraphtopology::SoundGraphTopology, soundnumberinput::SoundNumberInputId},
};

use super::{
    codegen::CodeGen,
    compilednumberinput::{CompiledNumberInputCache, CompiledNumberInputFunction},
};

// TODO: put one of these on the all-purpose inkwell worker thread

// An object to receive and serve requests for compiled number inputs,
// as well as stored cached artefacts according to their revision
struct Entry<'ctx> {
    cache: CompiledNumberInputCache<'ctx>,
    // TODO: info about how recently the entry was used,
    // in order to help clean things out efficiently.
    // Wait hang on, maybe reference counts suffice?
}

pub(crate) struct JitServer<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    artefacts: HashMap<(SoundNumberInputId, RevisionNumber), Entry<'ctx>>,
}

impl<'ctx> JitServer<'ctx> {
    pub(crate) fn new(inkwell_context: &'ctx inkwell::context::Context) -> JitServer<'ctx> {
        JitServer {
            inkwell_context,
            artefacts: HashMap::new(),
        }
    }

    pub(crate) fn get_compiled_number_input(
        &mut self,
        id: SoundNumberInputId,
        topology: &SoundGraphTopology,
    ) -> CompiledNumberInputFunction<'ctx> {
        let rev = topology.number_input(id).unwrap().get_revision();
        let key = (id, rev);
        let entry = self.artefacts.entry(key).or_insert_with(|| {
            let codegen = CodeGen::new(self.inkwell_context);
            let cache = codegen.compile_number_input(id, topology);
            Entry { cache }
        });
        entry.cache.make_function()
    }
}
