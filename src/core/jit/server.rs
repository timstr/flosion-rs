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

// TODO: how can compiled functions holding the 'ctx lifetime be used in
// the gui thread?
// - adding 'ctx throughout the ui would be akward, since the inkwell
//   context and the 'ctx lifetime associated with it is confined to a
//   scoped thread in soundgraph.rs. In principle, this could also run
//   a scoped gui thread, but that would mean a signficant inside-out
//   rewrite just to briefly access a jit-compiled function.
//   The gui also probably should remain on the main thread for
//   portability, since not all OS windowing systems allow for non-main
//   gui threads.
// - The underlying memory safety issue here is to ensure that any
//   inkwell resources used by the gui thread are dropped before
//   the inkwell thread exits. Given that the inkwell thread is
//   meant to be long-lived, this shouldn't be a huge issue.
// - A more realistic issue is that inkwell probably can't handle
//   compilation on different threads, and so the jit server needs
//   to perform compilation using a mechanism that delegates between
//   threads.
// - That thread separation of compilation from access would be well
//   served by something like a JitClient struct, which could
//    1. Send requests to the JitServer through e.g. a channel,
//       block until they're served, and return the requested artefact
//    2. Limit the scope of compiled artefacts to e.g. the inside of
//       a closure. This would be coupled with sending requests,
//       and could look something like:
//           jit_client.recv_compiled_number_input_with(|f| {
//               f.eval_standalone(&`mut some_temp_buffer);
//           });
//       Aaaaactually this probably isn't needed. Supposing that JitClient
//       already ensures (through its own means) that it outlives the
//       JitServer, it will suffice to have the artefact returned by jit
//       client store a lifetime to the jit client
//    3. Ensure the longevity of the inkwell context. This might
//       require using unsafe code, but could be achieved with something
//       as simple as a barrier at the end of the inkwell thread.
// - A separate issue here is that the ui will not be able to
//   provide an argument for the `context` argument of the existing
//   eval() method of a compiled number input function. It probably
//   makes sense to add a mechanism to mock these inputs, e.g.
//   provide a mock context that can provide spatially meaningful
//   time / phase / frequency / whatever values in place of actual
//   number sources. Since these are currently provided by function
//   pointers back into rust code, this should be feasible with
//   a simple trait to abstract between a real and mock context.
//   Additionally, it's clear that input values should be defined
//   where they're used, according to their interpretation as time /
//   frequency / phase / etc.

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
    pub(crate) fn serve(&self, topology: &SoundGraphTopology) {
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
}

impl<'ctx> Drop for JitServer<'ctx> {
    fn drop(&mut self) {
        println!("JitServer waiting to be dropped until client is also dropped");
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
            self.request_sender
                .send(JitClientRequest::PleaseCompile(id, revision))
                .unwrap();
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
