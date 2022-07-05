use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{
    context::{Context, SoundProcessorFrame, SoundStackFrame},
    samplefrequency::SAMPLE_FREQUENCY,
    scratcharena::ScratchArena,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundgraphtopology::SoundGraphTopology,
    soundprocessor::SoundProcessorId,
};

pub struct SoundEngine {
    topology: Arc<RwLock<SoundGraphTopology>>,
    keep_running: Arc<AtomicBool>,
    static_processor_cache: Vec<(SoundProcessorId, Option<SoundChunk>)>,
    scratch_space: ScratchArena,
}

pub enum PlaybackStatus {
    Continue,
    Stop,
}

impl SoundEngine {
    pub fn new() -> (SoundEngine, Arc<AtomicBool>) {
        let keep_running = Arc::new(AtomicBool::new(false));
        (
            SoundEngine {
                topology: Arc::new(RwLock::new(SoundGraphTopology::new())),
                keep_running: Arc::clone(&keep_running),
                static_processor_cache: Vec::new(),
                scratch_space: ScratchArena::new(),
            },
            keep_running,
        )
    }

    pub fn topology(&self) -> Arc<RwLock<SoundGraphTopology>> {
        Arc::clone(&self.topology)
    }

    pub fn run(&mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        set_current_thread_priority(ThreadPriority::Max).unwrap();

        for p in self.topology.read().sound_processors().values() {
            p.wrapper().on_start_processing();
        }

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            self.process_audio();
            self.scratch_space.cleanup();
            if !self.keep_running.load(Ordering::Relaxed) {
                break;
            }

            let now = Instant::now();
            if now > deadline {
                println!("WARNING: SoundEngine missed a deadline");
            } else {
                let delta = deadline.duration_since(now);
                spin_sleep::sleep(delta);
            }
            deadline += chunk_duration;
        }

        for p in self.topology.read().sound_processors().values() {
            p.wrapper().on_stop_processing();
        }
    }

    fn update_static_processor_cache(&mut self) {
        let topology = self.topology.read();
        let mut remaining_static_proc_ids: Vec<SoundProcessorId> = topology
            .sound_processors()
            .values()
            .filter_map(|proc_data| {
                if proc_data.wrapper().is_static() {
                    Some(proc_data.id())
                } else {
                    None
                }
            })
            .collect();
        fn depends_on_remaining_procs(
            proc_id: SoundProcessorId,
            remaining: &Vec<SoundProcessorId>,
            topology: &SoundGraphTopology,
        ) -> bool {
            let proc_data = topology.sound_processors().get(&proc_id).unwrap();
            for input_id in proc_data.inputs() {
                let input_data = topology.sound_inputs().get(&input_id).unwrap();
                if let Some(target_proc_id) = input_data.target() {
                    if remaining
                        .iter()
                        .find(|pid| **pid == target_proc_id)
                        .is_some()
                    {
                        return true;
                    }
                    if depends_on_remaining_procs(target_proc_id, remaining, topology) {
                        return true;
                    }
                }
            }
            return false;
        }

        self.static_processor_cache.clear();

        loop {
            let next_avail_proc = remaining_static_proc_ids.iter().position(|pid| {
                !depends_on_remaining_procs(*pid, &remaining_static_proc_ids, &*topology)
            });
            match next_avail_proc {
                Some(idx) => {
                    let pid = remaining_static_proc_ids.remove(idx);
                    self.static_processor_cache.push((pid, None))
                }
                None => break,
            }
        }
    }

    fn process_audio(&mut self) {
        // TODO: find a way to do this more efficiently, e.g. as part of modifying topology on other thread
        self.update_static_processor_cache();

        let topology = self.topology.read();
        debug_assert!(
            self.static_processor_cache
                .iter()
                .find(|(pid, _)| topology.sound_processors().get(pid).is_none())
                .is_none(),
            "The cached static processor ids should all exist"
        );
        debug_assert!(
            topology
                .sound_processors()
                .iter()
                .filter_map(|(pid, pdata)| if pdata.wrapper().is_static() {
                    Some(*pid)
                } else {
                    None
                })
                .find(|pid| self
                    .static_processor_cache
                    .iter()
                    .find(|(i, _)| *i == *pid)
                    .is_none())
                .is_none(),
            "All static processors should be in the cache"
        );

        for (_, ch) in &mut self.static_processor_cache {
            *ch = None;
        }

        for idx in 0..self.static_processor_cache.len() {
            let pid = self.static_processor_cache[idx].0;
            let proc_data = topology.sound_processors().get(&pid).unwrap();
            debug_assert!(proc_data.wrapper().is_static());
            let stack = vec![SoundStackFrame::Processor(SoundProcessorFrame {
                id: pid,
                state_index: 0,
            })];
            // NOTE: starting with an empty stack here means that upstream
            // number sources will all be out of scope. It's probably safe
            // to allow upstream number sources as long as they are on a
            // unique path
            let context = Context::new(
                topology.sound_processors(),
                topology.sound_inputs(),
                topology.number_sources(),
                topology.number_inputs(),
                &self.static_processor_cache,
                stack,
                &self.scratch_space,
            );
            let mut chunk = SoundChunk::new();
            proc_data.wrapper().process_audio(&mut chunk, context);
            self.static_processor_cache[idx].1 = Some(chunk);
        }
    }
}
