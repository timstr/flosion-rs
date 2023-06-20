use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{
    nodegen::NodeGen, scratcharena::ScratchArena, stategraph::StateGraph,
    stategraphedit::StateGraphEdit,
};

use crate::core::{
    engine::stategraphvalidation::state_graph_matches_topology, samplefrequency::SAMPLE_FREQUENCY,
    sound::soundgraphtopology::SoundGraphTopology, soundchunk::CHUNK_SIZE,
};

pub(crate) fn create_sound_engine<'ctx>(
    inkwell_context: &'ctx inkwell::context::Context,
) -> (SoundEngineInterface<'ctx>, SoundEngineRunner<'ctx>) {
    let keep_running = Arc::new(AtomicBool::new(true));
    let (sender, receiver) = channel::<StateGraphEdit<'ctx>>();

    let se_interface = SoundEngineInterface {
        inkwell_context,
        current_topology: SoundGraphTopology::new(),
        keep_running: Arc::clone(&keep_running),
        edit_queue: sender,
    };

    let se_runner = SoundEngineRunner {
        keep_running,
        edit_queue: receiver,
    };

    (se_interface, se_runner)
}

pub(crate) struct SoundEngineInterface<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    current_topology: SoundGraphTopology,
    keep_running: Arc<AtomicBool>,
    edit_queue: Sender<StateGraphEdit<'ctx>>,
}

impl<'ctx> SoundEngineInterface<'ctx> {
    pub(crate) fn update(&mut self, new_topology: SoundGraphTopology) {
        // TODO: diff current and new topology and create a list of fine-grained state graph edits
        // HACK deleting everything and then adding it back
        for proc in self.current_topology.sound_processors().values() {
            if proc.instance().is_static() {
                self.edit_queue
                    .send(StateGraphEdit::RemoveStaticSoundProcessor(proc.id()))
                    .unwrap();
            }
        }
        // all should be deleted now
        #[cfg(debug_assertions)]
        {
            self.edit_queue
                .send(StateGraphEdit::DebugInspection(Box::new(
                    |sg: &StateGraph<'ctx>| {
                        debug_assert!(sg.static_nodes().is_empty());
                    },
                )))
                .unwrap();
        }

        // Add back static processors with populated inputs
        let nodegen = NodeGen::new(&self.current_topology, self.inkwell_context);
        for proc in self.current_topology.sound_processors().values() {
            if proc.instance().is_static() {
                let node = proc.instance_arc().make_node(&nodegen);
                self.edit_queue
                    .send(StateGraphEdit::AddStaticSoundProcessor(node));
            }
        }

        // topology and state graph should match now
        #[cfg(debug_assertions)]
        {
            let topo_clone = new_topology.clone();
            self.edit_queue
                .send(StateGraphEdit::DebugInspection(Box::new(
                    |sg: &StateGraph<'ctx>| {
                        let topo = topo_clone;
                        debug_assert!(state_graph_matches_topology(sg, &topo));
                    },
                )))
                .unwrap();
        }

        self.current_topology = new_topology;
    }
}

impl<'ctx> Drop for SoundEngineInterface<'ctx> {
    fn drop(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}

pub(crate) struct SoundEngineRunner<'ctx> {
    keep_running: Arc<AtomicBool>,
    edit_queue: Receiver<StateGraphEdit<'ctx>>,
}

impl<'ctx> SoundEngineRunner<'ctx> {
    pub(crate) fn run(self) {
        let mut se = SoundEngine {
            keep_running: self.keep_running,
            edit_queue: self.edit_queue,
            deadline_warning_issued: false,
        };
        se.run();
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

impl<'ctx> SoundEngine<'ctx> {
    thread_local! {
        static SCRATCH_SPACE: ScratchArena = ScratchArena::new();
    }

    fn run(&mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        set_current_thread_priority(ThreadPriority::Max).unwrap();

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
