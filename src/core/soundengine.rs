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

use crate::core::stategraphvalidation::state_graph_matches_topology;

use super::{
    samplefrequency::SAMPLE_FREQUENCY, scratcharena::ScratchArena, soundchunk::CHUNK_SIZE,
    soundgraphedit::SoundGraphEdit, soundgraphtopology::SoundGraphTopology, stategraph::StateGraph,
};

pub(super) struct SoundEngineInterface {
    keep_running: Arc<AtomicBool>,
    edit_queue: Sender<SoundGraphEdit>,
    join_handle: Option<JoinHandle<()>>,
}

impl SoundEngineInterface {
    pub(super) fn make_edit(&self, edit: SoundGraphEdit) {
        self.edit_queue.send(edit).unwrap();
    }
}

impl Drop for SoundEngineInterface {
    fn drop(&mut self) {
        self.keep_running.store(false, Ordering::SeqCst);
        self.join_handle.take().unwrap().join().unwrap();
    }
}

pub(super) struct SoundEngine {
    keep_running: Arc<AtomicBool>,
    edit_queue: Receiver<SoundGraphEdit>,
    deadline_warning_issued: bool,
}

impl SoundEngine {
    thread_local! {
        static SCRATCH_SPACE: ScratchArena = ScratchArena::new();
    }

    pub(super) fn spawn() -> SoundEngineInterface {
        let keep_running = Arc::new(AtomicBool::new(true));
        let keep_running_also = Arc::clone(&keep_running);
        let (sender, receiver) = channel();
        let join_handle = thread::spawn(move || {
            // NOTE: state_graph is not Send, and so must be constructed on the audio thread
            let mut se = SoundEngine {
                keep_running: keep_running_also,
                edit_queue: receiver,
                deadline_warning_issued: false,
            };
            se.run();
        });
        SoundEngineInterface {
            keep_running,
            edit_queue: sender,
            join_handle: Some(join_handle),
        }
    }

    fn run(&mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        set_current_thread_priority(ThreadPriority::Max).unwrap();

        let context = inkwell::context::Context::create();
        let mut topology = SoundGraphTopology::new();
        let mut state_graph = StateGraph::new();

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            self.flush_updates(&mut state_graph, &mut topology, &context);

            self.process_audio(&state_graph, &topology);
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

    fn flush_updates<'ctx>(
        &mut self,
        state_graph: &mut StateGraph<'ctx>,
        topology: &mut SoundGraphTopology,
        context: &'ctx inkwell::context::Context,
    ) {
        while let Ok(edit) = self.edit_queue.try_recv() {
            println!("SoundEngine: {}", edit.name());
            let stategraph_first = match edit {
                SoundGraphEdit::AddSoundProcessor(_) => false,
                SoundGraphEdit::RemoveSoundProcessor(_) => true,
                SoundGraphEdit::AddSoundInput(_) => false,
                SoundGraphEdit::RemoveSoundInput(_, _) => true,
                SoundGraphEdit::AddSoundInputKey(_, _) => false,
                SoundGraphEdit::RemoveSoundInputKey(_, _) => true,
                SoundGraphEdit::ConnectSoundInput(_, _) => false,
                SoundGraphEdit::DisconnectSoundInput(_) => false,
                SoundGraphEdit::AddNumberSource(_) => false,
                SoundGraphEdit::RemoveNumberSource(_, _) => true,
                SoundGraphEdit::AddNumberInput(_) => false,
                SoundGraphEdit::RemoveNumberInput(_, _) => true,
                SoundGraphEdit::ConnectNumberInput(_, _) => false,
                SoundGraphEdit::DisconnectNumberInput(_) => false,
            };
            if stategraph_first {
                state_graph.make_edit(edit.clone(), topology, context);
                topology.make_edit(edit);
            } else {
                topology.make_edit(edit.clone());
                state_graph.make_edit(edit, topology, context);
            }
            debug_assert!(state_graph_matches_topology(state_graph, topology));
        }
    }

    fn process_audio(&mut self, state_graph: &StateGraph, topology: &SoundGraphTopology) {
        Self::SCRATCH_SPACE.with(|scratch_space| {
            for entry_point in state_graph.entry_points() {
                entry_point.invoke_externally(topology, scratch_space);
            }
        });
    }
}
