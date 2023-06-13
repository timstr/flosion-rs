use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{scratcharena::ScratchArena, stategraph::StateGraph, stategraphedit::StateGraphEdit};

use crate::core::{
    jit::jitcontext::JitContext, samplefrequency::SAMPLE_FREQUENCY,
    sound::soundgraphtopology::SoundGraphTopology, soundchunk::CHUNK_SIZE,
};

pub(crate) struct SoundEngineInterface<'ctx> {
    current_topology: SoundGraphTopology,
    keep_running: Arc<AtomicBool>,
    edit_queue: Sender<StateGraphEdit<'ctx>>,
    join_handle: Option<JoinHandle<()>>,
}

impl<'ctx> SoundEngineInterface<'ctx> {
    pub(crate) fn update(&mut self, new_topology: SoundGraphTopology) {
        // TODO: diff current and new topology and create a list of state
        // graph edits
        // TODO: as a simpler first pass, consider just regenerating the
        // entire state graph, e.g. replacing all state processors and
        // their targets
        todo!()
        // self.edit_queue.send(edit).unwrap();
        // self.current_topology = new_topology;
    }
}

impl<'ctx> Drop for SoundEngineInterface<'ctx> {
    fn drop(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
        self.join_handle.take().unwrap().join().unwrap();
    }
}

pub(crate) struct SoundEngine<'ctx> {
    // TODO: add a queue for whatever objects the state graph no longer needs
    // (e.g. removed sound processors, out-of-date compiled number inputs,
    // removed sound input targets, etc) which need to be dropped but should
    // ideally be dropped on a different thread to avoid spending audio
    // processing deallocating objects. Call it the garbage chute maybe.
    // Might require a dedicated trait so that Arc<dyn Garbage> or
    // Box<dyn Garbage> can be sent down the chute.
    keep_running: Arc<AtomicBool>,
    edit_queue: Receiver<StateGraphEdit<'ctx>>,
    deadline_warning_issued: bool,
}

pub(crate) fn spawn_sound_engine<'ctx>(
    jit_context: &'ctx JitContext,
) -> SoundEngineInterface<'ctx> {
    let keep_running = Arc::new(AtomicBool::new(true));
    let keep_running_also = Arc::clone(&keep_running);
    let (sender, receiver) = channel();
    let join_handle = thread::spawn(move || {
        // NOTE: state_graph is not Send, and so must be constructed on the audio thread
        // let mut se = SoundEngine {
        //     keep_running: keep_running_also,
        //     edit_queue: receiver,
        //     deadline_warning_issued: false,
        // };
        // se.run();
        // TODO: store only a single inkwell ExecutionEngine as part of a pre-constructed jit context,
        // and just hope that inkwell's JitFunction is Send
        // See notes in compilednumberinput.rs
        todo!()
    });
    SoundEngineInterface {
        current_topology: SoundGraphTopology::new(),
        keep_running,
        edit_queue: sender,
        join_handle: Some(join_handle),
    }
}

impl<'ctx> SoundEngine<'ctx> {
    thread_local! {
        static SCRATCH_SPACE: ScratchArena = ScratchArena::new();
    }

    fn run(&mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        set_current_thread_priority(ThreadPriority::Max).unwrap();

        let context = inkwell::context::Context::create();
        let mut state_graph = StateGraph::new();

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            self.flush_updates(&mut state_graph);

            self.process_audio(&state_graph);
            if !self.keep_running.load(Ordering::Relaxed) {
                break;
            }

            let now = Instant::now();
            if now > deadline {
                if !self.deadline_warning_issued {
                    println!("WARNING: SoundEngine missed a deadline");
                    self.deadline_warning_issued = true;
                }
            } else {
                self.deadline_warning_issued = false;
                let delta = deadline.duration_since(now);
                spin_sleep::sleep(delta);
            }
            deadline += chunk_duration;
        }
    }

    fn flush_updates(&mut self, state_graph: &mut StateGraph<'ctx>) {
        while let Ok(edit) = self.edit_queue.try_recv() {
            state_graph.make_edit(edit);
            // TODO: consider adding back a way to ensure that the state graph
            // matches the corresponding topology in debug builds
        }
    }

    fn process_audio(&mut self, state_graph: &StateGraph) {
        Self::SCRATCH_SPACE.with(|scratch_space| {
            for node in state_graph.static_nodes() {
                if node.is_entry_point() {
                    node.invoke_externally(scratch_space);
                }
            }
        });
    }
}
