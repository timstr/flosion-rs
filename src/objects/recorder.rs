use std::sync::{
    atomic::{self, AtomicBool},
    Arc,
};

use parking_lot::RwLock;

use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::InputOptions,
    soundprocessor::SoundProcessor,
    soundprocessortools::SoundProcessorTools,
    statetree::{ProcessorState, SingleInput, SingleInputNode, State},
};

const CHUNKS_PER_GROUP: usize = 64;
pub struct RecorderData {
    recorded_chunk_groups: RwLock<Vec<Vec<SoundChunk>>>,
    recording: AtomicBool,
}

impl RecorderData {
    pub fn new() -> Self {
        Self {
            recorded_chunk_groups: RwLock::new(vec![Vec::with_capacity(CHUNKS_PER_GROUP)]),
            recording: AtomicBool::new(false),
        }
    }
}

impl State for Arc<RecorderData> {
    fn reset(&mut self) {
        // Nothing to do
    }
}

pub struct Recorder {
    pub input: SingleInput,
    data: Arc<RecorderData>,
}

impl Recorder {
    pub fn start_recording(&self) {
        self.data.recording.store(true, atomic::Ordering::Relaxed)
    }

    pub fn stop_recording(&self) {
        self.data.recording.store(false, atomic::Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.data.recording.load(atomic::Ordering::Relaxed)
    }

    pub fn copy_audio(&self) -> Vec<(f32, f32)> {
        let chunk_groups = self.data.recorded_chunk_groups.read();
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
        *self.data.recorded_chunk_groups.write() = vec![Vec::with_capacity(CHUNKS_PER_GROUP)];
    }

    pub fn recording_length(&self) -> usize {
        let mut n: usize = 0;
        for ch in &*self.data.recorded_chunk_groups.read() {
            n += CHUNK_SIZE * ch.len();
        }
        n
    }
}

impl SoundProcessor for Recorder {
    const IS_STATIC: bool = true;

    type StateType = Arc<RecorderData>;

    type InputType = SingleInput;

    fn new(mut tools: SoundProcessorTools) -> Self {
        let r = Recorder {
            input: SingleInput::new(
                InputOptions {
                    interruptible: false,
                    realtime: true,
                },
                &mut tools,
            ),
            data: Arc::new(RecorderData::new()),
        };
        debug_assert!(r.recording_length() == 0);
        r
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        Arc::clone(&self.data)
    }

    fn process_audio(
        state: &mut ProcessorState<Arc<RecorderData>>,
        inputs: &mut SingleInputNode,
        dst: &mut SoundChunk,
        ctx: Context,
    ) {
        inputs.step(state, dst, &ctx);
        let recording = state.recording.load(atomic::Ordering::Relaxed);
        if !recording {
            return;
        }
        let mut groups = state.recorded_chunk_groups.write();
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
}

impl WithObjectType for Recorder {
    const TYPE: ObjectType = ObjectType::new("recorder");
}
