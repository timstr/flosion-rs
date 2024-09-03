use std::{collections::HashMap, sync::Arc};

use hashrevise::RevisionHash;
use parking_lot::{Condvar, Mutex, RwLock};

use crate::core::sound::{expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology};

use super::{
    codegen::CodeGen,
    compiledexpression::{CompiledExpressionArtefact, CompiledExpressionFunction},
};

// An object to receive and serve requests for compiled expressions,
// as well as stored cached artefacts according to their revision
struct Entry<'ctx> {
    artefact: CompiledExpressionArtefact<'ctx>,
    // TODO: info about how recently the entry was used,
    // in order to help clean things out efficiently.
    // Wait hang on, maybe reference counts suffice?
    // In that case, consider exposing the use_count
    // of the artefact's inner Arc
}

struct Cache<'ctx> {
    artefacts: HashMap<(SoundExpressionId, RevisionHash), Entry<'ctx>>,
}

impl<'ctx> Cache<'ctx> {
    fn new() -> Cache<'ctx> {
        Cache {
            artefacts: HashMap::new(),
        }
    }

    fn get_compiled_expression(
        &self,
        id: SoundExpressionId,
        revision: RevisionHash,
    ) -> Option<CompiledExpressionFunction<'ctx>> {
        let key = (id, revision);
        self.artefacts.get(&key).map(|c| c.artefact.make_function())
    }

    fn insert(
        &mut self,
        input_id: SoundExpressionId,
        revision_number: RevisionHash,
        artefact: CompiledExpressionArtefact<'ctx>,
    ) {
        self.artefacts
            .insert((input_id, revision_number), Entry { artefact });
    }

    fn len(&self) -> usize {
        self.artefacts.len()
    }

    // TODO: methods to age-out and clean up the cache based on usage
}

pub struct JitServerBuilder {
    // NOTE: 'static lifetime is used here to allow clients to be unaware of the
    //
    cache: Arc<RwLock<Cache<'static>>>,
}

impl JitServerBuilder {
    pub(crate) fn new() -> JitServerBuilder {
        let cache = Arc::new(RwLock::new(Cache::new()));
        let mutex_and_cond_var = Arc::new((Mutex::new(false), Condvar::new()));

        JitServerBuilder {
            cache: Arc::clone(&cache),
        }
    }

    pub(crate) fn build_server<'ctx>(
        self,
        inkwell_context: &'ctx inkwell::context::Context,
    ) -> JitServer<'ctx> {
        let JitServerBuilder { cache } = self;
        assert!(cache.read().artefacts.is_empty());
        // SAFETY: the cache here is intended to contain data referencing the inkwell
        // context on its own thread, after being passed from another thread.
        // At this point in time, the cache contains no such data yet.
        // This cache is shared by a client which will be able to access
        // inkwell data from the original thread where it won't have an associated
        // lifetime. In order to be memory safe, the client must be dropped and no
        // inkwell data may be held on the other thread before the server is dropped.
        // For this reason, a condition variable is used to ensure that the server
        // waits for the client to be dropped.
        let nonstatic_cache: Arc<RwLock<Cache<'ctx>>> = unsafe { std::mem::transmute(cache) };
        JitServer {
            inkwell_context,
            cache: nonstatic_cache,
        }
    }
}

// TODO: replace with just cache
pub(crate) struct JitServer<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    cache: Arc<RwLock<Cache<'ctx>>>,
}

impl<'ctx> JitServer<'ctx> {
    pub(crate) fn get_compiled_expression(
        &self,
        id: SoundExpressionId,
        topology: &SoundGraphTopology,
    ) -> CompiledExpressionFunction<'ctx> {
        let mut cache = self.cache.write();
        let input_data = topology.expression(id).unwrap();
        let revision = input_data.get_revision();
        cache
            .artefacts
            .entry((id, revision))
            .or_insert_with(|| {
                let codegen = CodeGen::new(self.inkwell_context);
                let artefact = codegen.compile_expression(id, topology);
                Entry { artefact }
            })
            .artefact
            .make_function()
    }
}
