use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
        Arc,
    },
    time::{Duration, Instant},
};

use hashstash::{stash_clone_with_context, ObjectHash, Stash};

use super::{
    diffgraph::diff_sound_graph,
    garbage::{new_garbage_disposer, Garbage, GarbageChute, GarbageDisposer},
    scratcharena::ScratchArena,
    stategraph::StateGraph,
    stategraphedit::StateGraphEdit,
};

use crate::core::{
    expression::expressionobject::ExpressionObjectFactory,
    jit::{argumentstack::ArgumentStack, cache::JitCache},
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{soundgraph::SoundGraph, soundobject::SoundObjectFactory},
    soundchunk::CHUNK_SIZE,
    stashing::{StashingContext, UnstashingContext},
};

/// A thread-safe signaling mechanism used to communicate
/// 'keep going' or 'stop', to allow infinite loops on
/// multiple threads to terminate together. Uses an atomic
/// boolean internally.
pub(crate) struct StopButton(Arc<AtomicBool>);

impl StopButton {
    /// Create a new StopButton in its default, not-yet-stopped
    /// state. To share the same stop button, simply clone it.
    pub(crate) fn new() -> StopButton {
        StopButton(Arc::new(AtomicBool::new(false)))
    }

    /// Push the stop button. After this point, all clones of the
    /// stop button on all threads will see 'was_stopped()'
    /// return true.
    pub(crate) fn stop(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Check whether the stop button has been pushed. Use this in
    /// a loop condition to know when a different thread wants you
    /// to exit the loop.
    pub(crate) fn was_stopped(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Clone for StopButton {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

/// Constructs a new sound engine, interface for the sound engine,
/// and a garbage disposer.
///
/// The sound engine itself is intended for direct audio processing
/// on a high-priority thread via its `run` method. Use the provided
/// stop button to cause the `run` method to exit.
///
/// The sound engine interface serves to receive changes to the
/// sound graph that the audio thread is modeling, and to
/// relate those changes to the sound engine in an efficient and
/// pre-allocated manner.
///
/// The garbage disposer receives stale things from the sound engine
/// that may require heap deallocation, and are thus not realtime-
/// safe to drop on the audio thread. This needs to be emptied
/// periodically while changes are being made via the sound engine
/// interface.
pub(crate) fn create_sound_engine<'ctx>(
    stop_button: &StopButton,
) -> (
    SoundEngineInterface<'ctx>,
    SoundEngine<'ctx>,
    GarbageDisposer<'ctx>,
) {
    let edit_queue_size = 1024;
    let (edit_sender, edit_receiver) = sync_channel::<StateGraphEdit<'ctx>>(edit_queue_size);
    let (garbage_chute, garbage_disposer) = new_garbage_disposer();

    let current_graph = SoundGraph::new();
    let current_hash = ObjectHash::from_stashable_and_context(
        &current_graph,
        StashingContext::new_checking_recompilation(),
    );
    let se_interface = SoundEngineInterface {
        current_graph,
        current_hash,
        stop_button: stop_button.clone(),
        edit_queue: edit_sender,
    };

    let se = SoundEngine {
        state_graph: StateGraph::new(),
        stop_button: stop_button.clone(),
        edit_queue: edit_receiver,
        deadline_warning_issued: false,
        garbage_chute,
    };

    (se_interface, se, garbage_disposer)
}

/// An intermediate object between a series of changing SoundGraph
/// instances and the SoundEngine running on a separate thread, which is
/// intended to audibly model those changes as they come. SoundEngineInterface
/// compiles changes to the graph for the SoundEngine and sends the
/// compiled results to the audio thread, where they are patched in as
/// efficiently as possible and without any heap allocation or deallocation
/// on the audio thread.
///
/// Note that dropping the SoundEngineInterface will cause the SoundEngine
/// to stop running.
pub(crate) struct SoundEngineInterface<'ctx> {
    current_graph: SoundGraph,
    current_hash: ObjectHash,
    stop_button: StopButton,
    edit_queue: SyncSender<StateGraphEdit<'ctx>>,
}

impl<'ctx> SoundEngineInterface<'ctx> {
    /// Update the SoundEngine on the separate thread to model and produce
    /// audio according to the given graph. Changes between this and
    /// the most recent graph are compiled and sent to the audio thread.
    pub(crate) fn update(
        &mut self,
        new_graph: &SoundGraph,
        jit_cache: &JitCache<'ctx>,
        stash: &Stash,
        sound_object_factory: &SoundObjectFactory,
        expression_object_factory: &ExpressionObjectFactory,
    ) -> Result<(), ()> {
        let new_revision = ObjectHash::from_stashable_and_context(
            new_graph,
            StashingContext::new_checking_recompilation(),
        );

        if new_revision == self.current_hash {
            return Ok(());
        }

        let edits = diff_sound_graph(&self.current_graph, &new_graph, jit_cache);

        for edit in edits {
            match self.edit_queue.try_send(edit) {
                Err(TrySendError::Full(_)) => panic!("State graph edit queue overflow!"),
                Err(TrySendError::Disconnected(_)) => {
                    println!(
                        "State graph thread is no longer running, \
                            sound engine update thread is exiting"
                    );
                    return Err(());
                }
                Ok(_) => (),
            }
        }

        let (cloned_graph, _) = stash_clone_with_context(
            new_graph,
            stash,
            StashingContext::new_stashing_normally(),
            UnstashingContext::new(sound_object_factory, expression_object_factory),
        )
        .unwrap();

        self.current_graph = cloned_graph;
        self.current_hash = new_revision;

        Ok(())
    }
}

impl<'ctx> Drop for SoundEngineInterface<'ctx> {
    fn drop(&mut self) {
        self.stop_button.stop();
    }
}

/// SoundEngine is directly responsible for actually invoking sound
/// processors to produce audio on the high-priority audio thread.
/// Simply call the `run()` method on a high-priority thread, and it
/// will perpetually produce audio until the stop button is pressed
/// (for example, if the SoundEngineInterface it was created with is
/// dropped).
pub(crate) struct SoundEngine<'ctx> {
    /// The state graph, containing all compiled sound processors,
    /// their compiled components, and their states
    state_graph: StateGraph<'ctx>,

    /// The stop button describing when to exit the audio loop due
    /// to things happening on other threads
    stop_button: StopButton,

    /// Inbound edits to the state graph, received from diffing and
    /// compiling graphs in the associated SoundEngineInterface.
    edit_queue: Receiver<StateGraphEdit<'ctx>>,

    /// Has a warning been issued that recent audio updates are
    /// behind schedule? Used to prevent spam
    deadline_warning_issued: bool,

    /// Garbage chute for sending away stale and unwanted data that
    /// is being replaced, to avoid heap deallocation happening on
    /// the audio  thread.
    garbage_chute: GarbageChute<'ctx>,
}

impl<'ctx> SoundEngine<'ctx> {
    thread_local! {
        /// Thread-local pool of buffers of f32 data, intended
        /// to be used briefly and then reused without reallocation.
        static SCRATCH_SPACE: ScratchArena = ScratchArena::new();
    }

    /// Process audio in realtime. Internally, this builds a StateGraph,
    /// receives edits to that StateGraph from the SoundEngineInterface,
    /// and invokes the nodes in the state graph regularly according to
    /// a high-precision timer.
    pub(crate) fn run(mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            // Receive and incorporate any state graph edits from the SoundEngineInterface
            self.flush_updates();

            // Invoke the sound processors
            self.process_audio();
            if self.stop_button.was_stopped() {
                break;
            }

            let now = Instant::now();
            if now > deadline {
                // If we just fell behind schedule, issue a warning
                // because audio dropouts are happening.
                if !self.deadline_warning_issued {
                    println!("WARNING: SoundEngine missed a deadline");
                    self.deadline_warning_issued = true;
                }
            } else {
                // If we're on schedule, sleep for precisely the
                // amount of time remaining until the next chunk
                // needs to start.
                self.deadline_warning_issued = false;
                let delta = deadline.duration_since(now);
                spin_sleep::sleep(delta);
            }
            deadline += chunk_duration;
        }

        // Throw out the state graph to ensure resource cleanup (particularly of
        // LLVM resources) happens on the correct thread
        self.state_graph.toss(&self.garbage_chute);
    }

    /// Receive and incorporate any edits to the given state graph from
    /// the given queue. Toss any old data down the given garbage chute.
    fn flush_updates(&mut self) {
        while let Ok(edit) = self.edit_queue.try_recv() {
            self.state_graph.make_edit(edit, &self.garbage_chute);
        }
    }

    /// Invoke all static sound processors in the state graph.
    /// This ensures that static processors are always update, and
    /// the dynamic processor nodes in their dependencies will
    /// be invoked recursively from there.
    fn process_audio(&mut self) {
        Self::SCRATCH_SPACE.with(|scratch_space| {
            // TODO: preserve this (to save allocation) between calls
            let argument_stack = ArgumentStack::new();
            for node in self.state_graph.static_processors() {
                // Invoke any node that is an entry point. An entry point
                // is a top-level shared node with no inputs connected
                // to it, so this effectively processes from the roots up.
                if node.is_entry_point() {
                    node.invoke_externally(scratch_space, &argument_stack);
                }
            }
        });
    }
}
