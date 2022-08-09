use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use thread_priority::{set_current_thread_priority, ThreadPriority};

use crate::core::{context::Context, soundchunk::SoundChunk};

use super::{
    samplefrequency::SAMPLE_FREQUENCY, scratcharena::ScratchArena, soundchunk::CHUNK_SIZE,
    soundgraphtopology::SoundGraphTopology,
};

pub struct SoundEngine {
    topology: Arc<RwLock<SoundGraphTopology>>,
    keep_running: Arc<AtomicBool>,
}

impl SoundEngine {
    thread_local! {
        static SCRATCH_SPACE: ScratchArena = ScratchArena::new();
    }

    pub fn new() -> (SoundEngine, Arc<AtomicBool>) {
        let keep_running = Arc::new(AtomicBool::new(false));
        (
            SoundEngine {
                topology: Arc::new(RwLock::new(SoundGraphTopology::new())),
                keep_running: Arc::clone(&keep_running),
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

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            self.process_audio();
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
    }

    fn process_audio(&mut self) {
        let topology = self.topology.read();
        debug_assert!(
            topology.static_processors().iter().all(|sp| topology
                .sound_processors()
                .get(&sp.processor_id())
                .is_some()),
            "The cached static processor ids should all exist"
        );
        debug_assert!(
            topology
                .sound_processors()
                .iter()
                .filter_map(|(pid, pdata)| if pdata.processor().is_static() {
                    Some(*pid)
                } else {
                    None
                })
                .all(|pid| topology
                    .static_processors()
                    .iter()
                    .find(|sp| pid == sp.processor_id())
                    .is_some()),
            "All static processors should be in the cache"
        );

        for cache in topology.static_processors() {
            *cache.output().write() = None;
        }

        for cache in topology.static_processors() {
            let mut ch = SoundChunk::new();
            Self::SCRATCH_SPACE.with(|scratch_space| {
                cache.tree().write().process_audio(
                    &mut ch,
                    Context::new(cache.processor_id(), &*topology, scratch_space),
                );
            });
            *cache.output().write() = Some(ch);
        }
    }
}
