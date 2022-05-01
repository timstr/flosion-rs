use std::sync::atomic::{self, AtomicBool};

use parking_lot::RwLock;

use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, TypedGraphObject},
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::{InputOptions, SingleSoundInputHandle},
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
};

const CHUNKS_PER_GROUP: usize = 64;

pub struct Recorder {
    pub input: SingleSoundInputHandle,
    recorded_chunk_groups: RwLock<Vec<Vec<SoundChunk>>>,
    recording: AtomicBool,
}

impl Recorder {
    pub fn start_recording(&self) {
        self.recording.store(true, atomic::Ordering::Relaxed)
    }

    pub fn stop_recording(&self) {
        self.recording.store(false, atomic::Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(atomic::Ordering::Relaxed)
    }

    pub fn copy_audio(&self) -> Vec<(f32, f32)> {
        let chunk_groups = self.recorded_chunk_groups.read();
        let n = self.recording_length();
        // let chunk_groups = self.chunk_groups.read();
        // let cgs: &Vec<Vec<SoundChunk>> = &*chunk_groups;
        let mut v: Vec<(f32, f32)> = Vec::with_capacity(n);
        for cg in &*chunk_groups {
            for c in cg {
                for i in 0..CHUNK_SIZE {
                    v.push((c.l[i], c.r[i]));
                }
            }
        }
        debug_assert!(v.len() == n);
        v
    }

    pub fn clear_recording(&self) {
        *self.recorded_chunk_groups.write() = vec![Vec::with_capacity(CHUNKS_PER_GROUP)];
    }

    pub fn recording_length(&self) -> usize {
        let mut n: usize = 0;
        for ch in &*self.recorded_chunk_groups.read() {
            n += CHUNK_SIZE * ch.len();
        }
        n
    }
}

pub struct RecorderState {}

impl Default for RecorderState {
    fn default() -> Self {
        Self {}
    }
}

impl SoundState for RecorderState {
    fn reset(&mut self) {}
}

impl StaticSoundProcessor for Recorder {
    type StateType = RecorderState;

    fn new(tools: &mut SoundProcessorTools<'_, RecorderState>) -> Recorder {
        let r = Recorder {
            input: tools
                .add_single_sound_input(InputOptions {
                    interruptible: false,
                    realtime: true,
                })
                .0,
            recorded_chunk_groups: RwLock::new(vec![Vec::with_capacity(CHUNKS_PER_GROUP)]),
            recording: AtomicBool::new(false),
        };
        debug_assert!(r.recording_length() == 0);
        r
    }

    fn process_audio(
        &self,
        dst: &mut SoundChunk,
        mut context: ProcessorContext<'_, RecorderState>,
    ) {
        context.step_single_input(&self.input, dst);
        let recording = self.recording.load(atomic::Ordering::Relaxed);
        if !recording {
            return;
        }
        let mut groups = self.recorded_chunk_groups.write();
        debug_assert!(groups.len() >= 1);
        let last_group = groups.last_mut().unwrap();
        if last_group.len() < last_group.capacity() {
            last_group.push(dst.clone());
        } else {
            let mut new_group = Vec::<SoundChunk>::with_capacity(CHUNKS_PER_GROUP);
            new_group.push(dst.clone());
            groups.push(new_group);
        }
    }

    fn produces_output(&self) -> bool {
        true
    }
}

impl TypedGraphObject for Recorder {
    const TYPE: ObjectType = ObjectType::new("recorder");
}
