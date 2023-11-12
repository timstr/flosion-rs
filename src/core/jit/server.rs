use std::{
    collections::HashMap,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
};

use parking_lot::{Condvar, Mutex, RwLock};

use crate::core::{
    revision::revision::RevisionNumber,
    sound::{soundgraphtopology::SoundGraphTopology, soundnumberinput::SoundNumberInputId},
};

use super::{
    codegen::CodeGen,
    compilednumberinput::{CompiledNumberInput, CompiledNumberInputFunction},
};

// TODO: put one of these on the all-purpose inkwell worker thread

// An object to receive and serve requests for compiled number inputs,
// as well as stored cached artefacts according to their revision
struct Entry<'ctx> {
    artefact: CompiledNumberInput<'ctx>,
    // TODO: info about how recently the entry was used,
    // in order to help clean things out efficiently.
    // Wait hang on, maybe reference counts suffice?
    // In that case, consider exposing the use_count
    // of the artefact's inner Arc
}

struct Cache<'ctx> {
    artefacts: HashMap<(SoundNumberInputId, RevisionNumber), Entry<'ctx>>,
}

impl<'ctx> Cache<'ctx> {
    fn new() -> Cache<'ctx> {
        Cache {
            artefacts: HashMap::new(),
        }
    }

    fn get_compiled_number_input(
        &self,
        id: SoundNumberInputId,
        revision: RevisionNumber,
    ) -> Option<CompiledNumberInputFunction<'ctx>> {
        let key = (id, revision);
        self.artefacts.get(&key).map(|c| c.artefact.make_function())
    }

    fn insert(
        &mut self,
        input_id: SoundNumberInputId,
        revision_number: RevisionNumber,
        artefact: CompiledNumberInput<'ctx>,
    ) {
        self.artefacts
            .insert((input_id, revision_number), Entry { artefact });
    }

    // TODO: methods to age-out and clean up the cache based on usage
}

pub struct JitServerBuilder {
    // NOTE: 'static lifetime is used here to allow clients to be unaware of the
    //
    cache: Arc<RwLock<Cache<'static>>>,

    // Used to block the jit server from being dropped until
    // the client has also been dropped
    mutex_and_cond_var: Arc<(Mutex<bool>, Condvar)>,

    client_receiver: Receiver<JitClientRequest>,
}

impl JitServerBuilder {
    pub(crate) fn new() -> (JitServerBuilder, JitClient) {
        let cache = Arc::new(RwLock::new(Cache::new()));
        let mutex_and_cond_var = Arc::new((Mutex::new(false), Condvar::new()));
        let (client_sender, client_receiver) = sync_channel(256);
        (
            JitServerBuilder {
                cache: Arc::clone(&cache),
                mutex_and_cond_var: Arc::clone(&mutex_and_cond_var),
                client_receiver: client_receiver,
            },
            JitClient {
                request_sender: client_sender,
                cache,
                mutex_and_cond_var,
            },
        )
    }

    pub(crate) fn build_server<'ctx>(
        self,
        inkwell_context: &'ctx inkwell::context::Context,
    ) -> JitServer<'ctx> {
        let JitServerBuilder {
            cache,
            mutex_and_cond_var,
            client_receiver,
        } = self;
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
            client_receiver,
            mutex_and_cond_var,
        }
    }
}

pub(crate) struct JitServer<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    cache: Arc<RwLock<Cache<'ctx>>>,
    client_receiver: Receiver<JitClientRequest>,
    mutex_and_cond_var: Arc<(Mutex<bool>, Condvar)>,
}

impl<'ctx> JitServer<'ctx> {
    pub(crate) fn serve_pending_requests(&self, topology: &SoundGraphTopology) {
        let mut cache = self.cache.write();
        while let Ok(request) = self.client_receiver.try_recv() {
            let JitClientRequest::PleaseCompile(niid, revnum) = request;
            // TODO: distinguish between
            // - number inputs that have been requested and are waiting to be served
            // - number inputs that were responded to but don't exist
            // - number inputs that were responded to but have changed
            let Some(ni_data) = topology.number_input(niid) else {
                // input doesn't exist, too bad
                continue;
            };
            if ni_data.get_revision() != revnum {
                // input was changed, too bad
                continue;
            }
            let codegen = CodeGen::new(self.inkwell_context);
            let artefact = codegen.compile_number_input(niid, topology);
            cache.insert(niid, revnum, artefact);
        }
    }

    pub(crate) fn get_compiled_number_input(
        &self,
        id: SoundNumberInputId,
        topology: &SoundGraphTopology,
    ) -> CompiledNumberInputFunction<'ctx> {
        let mut cache = self.cache.write();
        let input_data = topology.number_input(id).unwrap();
        let revision = input_data.get_revision();
        cache
            .artefacts
            .entry((id, revision))
            .or_insert_with(|| {
                let codegen = CodeGen::new(self.inkwell_context);
                let artefact = codegen.compile_number_input(id, topology);
                Entry { artefact }
            })
            .artefact
            .make_function()
    }
}

impl<'ctx> Drop for JitServer<'ctx> {
    fn drop(&mut self) {
        let (mutex, condvar) = &*self.mutex_and_cond_var;
        let mut lock = mutex.lock();
        if !*lock {
            condvar.wait(&mut lock);
        }
        assert!(*lock);
        println!("JitServer dropped");
    }
}

pub(crate) enum JitClientRequest {
    PleaseCompile(SoundNumberInputId, RevisionNumber),
}

pub(crate) struct JitClient {
    request_sender: SyncSender<JitClientRequest>,
    cache: Arc<RwLock<Cache<'static>>>,
    mutex_and_cond_var: Arc<(Mutex<bool>, Condvar)>,
}

impl JitClient {
    pub(crate) fn get_compiled_number_input<'a>(
        &'a self,
        id: SoundNumberInputId,
        revision: RevisionNumber,
    ) -> Option<CompiledNumberInputFunction<'a>> {
        let f = self.cache.read().get_compiled_number_input(id, revision);
        if f.is_none() {
            match self
                .request_sender
                .try_send(JitClientRequest::PleaseCompile(id, revision))
            {
                Ok(_) => (),
                Err(_) => println!("JitClient failed to send request for compilation"),
            }
        }
        f
    }
}

impl Drop for JitClient {
    fn drop(&mut self) {
        println!("JitClient being dropped, notifying JitServer");
        let (mutex, condvar) = &*self.mutex_and_cond_var;
        let mut lock = mutex.lock();
        *lock = true;
        condvar.notify_one();
    }
}
