use std::collections::HashMap;

use crate::core::{
    revision::revision::RevisionNumber,
    sound::{soundgraphtopology::SoundGraphTopology, soundnumberinput::SoundNumberInputId},
};

use super::{
    codegen::CodeGen,
    compilednumberinput::{CompiledNumberInputCache, CompiledNumberInputFunction},
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
//               f.eval_standalone(&mut some_temp_buffer);
//           });
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

pub(crate) struct JitClient {
    // TODO
    // - how to connect to the JitServer running on the inkwell thread?
}

impl JitClient {
    // TODO
}
