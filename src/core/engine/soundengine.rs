use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender, TrySendError},
        Arc,
    },
    time::{Duration, Instant},
};

use super::{
    garbage::{new_garbage_disposer, Garbage, GarbageChute, GarbageDisposer},
    nodegen::NodeGen,
    scratcharena::ScratchArena,
    stategraph::StateGraph,
    stategraphedit::StateGraphEdit,
};

use crate::core::{
    engine::stategraphvalidation::state_graph_matches_topology, samplefrequency::SAMPLE_FREQUENCY,
    sound::soundgraphtopology::SoundGraphTopology, soundchunk::CHUNK_SIZE,
};

pub(crate) struct StopButton(Arc<AtomicBool>);

impl StopButton {
    pub(crate) fn new() -> StopButton {
        StopButton(Arc::new(AtomicBool::new(true)))
    }

    pub(crate) fn stop(&self) {
        self.0.store(false, Ordering::Relaxed);
    }
}

impl Clone for StopButton {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

pub(crate) fn create_sound_engine<'ctx>(
    inkwell_context: &'ctx inkwell::context::Context,
    stop_button: &StopButton,
) -> (
    SoundEngineInterface<'ctx>,
    SoundEngine<'ctx>,
    GarbageDisposer<'ctx>,
) {
    let keep_running = Arc::clone(&stop_button.0);
    let edit_queue_size = 1024;
    let (edit_sender, edit_receiver) = sync_channel::<StateGraphEdit<'ctx>>(edit_queue_size);
    let (garbage_chute, garbage_disposer) = new_garbage_disposer();

    let se_interface = SoundEngineInterface {
        inkwell_context,
        current_topology: SoundGraphTopology::new(),
        keep_running: Arc::clone(&keep_running),
        edit_queue: edit_sender,
    };

    let se = SoundEngine {
        keep_running,
        edit_queue: edit_receiver,
        deadline_warning_issued: false,
        garbage_chute,
    };

    (se_interface, se, garbage_disposer)
}

pub(crate) struct SoundEngineInterface<'ctx> {
    inkwell_context: &'ctx inkwell::context::Context,
    current_topology: SoundGraphTopology,
    keep_running: Arc<AtomicBool>,
    edit_queue: SyncSender<StateGraphEdit<'ctx>>,
}

impl<'ctx> SoundEngineInterface<'ctx> {
    pub(crate) fn update(&mut self, new_topology: SoundGraphTopology) -> Result<(), ()> {
        let do_it = || -> Result<(), TrySendError<StateGraphEdit<'ctx>>> {
            // topology and state graph should match
            #[cfg(debug_assertions)]
            {
                let topo_clone = self.current_topology.clone();
                self.edit_queue
                    .try_send(StateGraphEdit::DebugInspection(Box::new(
                        |sg: &StateGraph<'ctx>| {
                            let topo = topo_clone;
                            debug_assert!(
                                state_graph_matches_topology(sg, &topo),
                                "State graph failed to match topology before any updates were made"
                            );
                        },
                    )))?;
            }

            // TODO: diff current and new topology and create a list of fine-grained state graph edits
            // HACK deleting everything and then adding it back
            for proc in self.current_topology.sound_processors().values() {
                if proc.instance().is_static() {
                    self.edit_queue
                        .try_send(StateGraphEdit::RemoveStaticSoundProcessor(proc.id()))?;
                }
            }
            // all should be deleted now
            #[cfg(debug_assertions)]
            {
                self.edit_queue
                    .try_send(StateGraphEdit::DebugInspection(Box::new(
                        |sg: &StateGraph<'ctx>| {
                            debug_assert!(sg.static_nodes().is_empty());
                        },
                    )))?;
            }

            // Add back static processors with populated inputs
            let nodegen = NodeGen::new(&new_topology, self.inkwell_context);
            for proc in new_topology.sound_processors().values() {
                if proc.instance().is_static() {
                    let node = proc.instance_arc().make_node(&nodegen);
                    self.edit_queue
                        .try_send(StateGraphEdit::AddStaticSoundProcessor(node))?;
                }
            }

            // topology and state graph should still match
            #[cfg(debug_assertions)]
            {
                let topo_clone = new_topology.clone();
                self.edit_queue
                    .try_send(StateGraphEdit::DebugInspection(Box::new(
                        |sg: &StateGraph<'ctx>| {
                            let topo = topo_clone;
                            debug_assert!(
                                state_graph_matches_topology(sg, &topo),
                                "State graph no longer matches topology after applying updates"
                            );
                        },
                    )))?;
            }

            self.current_topology = new_topology;

            Ok(())
        };

        if let Err(err) = do_it() {
            match err {
                TrySendError::Full(_) => panic!("State graph edit queue overflow!"),
                TrySendError::Disconnected(_) => {
                    println!(
                        "State graph thread is no longer running, \
                        sound engine update thread is exiting"
                    );
                    return Err(());
                }
            }
        }

        Ok(())
    }
}

impl<'ctx> Drop for SoundEngineInterface<'ctx> {
    fn drop(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
    }
}

pub(crate) struct SoundEngine<'ctx> {
    keep_running: Arc<AtomicBool>,
    edit_queue: Receiver<StateGraphEdit<'ctx>>,
    deadline_warning_issued: bool,
    garbage_chute: GarbageChute<'ctx>,
}

impl<'ctx> SoundEngine<'ctx> {
    thread_local! {
        static SCRATCH_SPACE: ScratchArena = ScratchArena::new();
    }

    pub(crate) fn run(mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        let mut state_graph = StateGraph::new();

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            Self::flush_updates(&self.edit_queue, &mut state_graph, &self.garbage_chute);

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

        state_graph.toss(&self.garbage_chute);
    }

    fn flush_updates(
        edit_queue: &Receiver<StateGraphEdit<'ctx>>,
        state_graph: &mut StateGraph<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        while let Ok(edit) = edit_queue.try_recv() {
            state_graph.make_edit(edit, garbage_chute);
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
