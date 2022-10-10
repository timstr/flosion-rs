use std::sync::{
    atomic::{self, AtomicBool},
    Arc,
};

use parking_lot::RwLock;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    serialization::{Serializable, Serializer},
    soundbuffer::SoundBuffer,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::InputOptions,
    soundprocessor::{SoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{ProcessorState, SingleInput, SingleInputNode, State},
};

const CHUNKS_PER_GROUP: usize = 64;
pub struct RecorderData {
    recorded_chunk_groups: RwLock<Vec<SoundBuffer>>,
    recording: AtomicBool,
}

impl RecorderData {
    pub fn new(buf: SoundBuffer) -> Self {
        Self {
            recorded_chunk_groups: RwLock::new(vec![buf]),
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

    pub fn copy_audio(&self) -> SoundBuffer {
        let chunk_groups = self.data.recorded_chunk_groups.read();
        let mut b = SoundBuffer::new_with_capacity(chunk_groups.len() * CHUNKS_PER_GROUP);
        for cg in &*chunk_groups {
            for c in cg.chunks() {
                b.push_chunk(c);
            }
        }
        b
    }

    pub fn clear_recording(&self) {
        let buf = SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP);
        *self.data.recorded_chunk_groups.write() = vec![buf];
    }

    pub fn recording_length(&self) -> usize {
        let mut n: usize = 0;
        for ch in &*self.data.recorded_chunk_groups.read() {
            n += CHUNK_SIZE * ch.chunks().len();
        }
        n
    }
}

impl SoundProcessor for Recorder {
    const IS_STATIC: bool = true;

    type StateType = Arc<RecorderData>;

    type InputType = SingleInput;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let buf = match _init {
            ObjectInitialization::Archive(mut a) => SoundBuffer::deserialize(&mut a)?,
            _ => SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP),
        };
        let r = Recorder {
            input: SingleInput::new(InputOptions { realtime: true }, &mut tools),
            data: Arc::new(RecorderData::new(buf)),
        };
        debug_assert!(r.recording_length() == 0);
        Ok(r)
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
    ) -> StreamStatus {
        if inputs.needs_reset() {
            inputs.reset(0);
        }
        let s = inputs.step(state, dst, &ctx);
        let recording = state.recording.load(atomic::Ordering::Relaxed);
        if !recording || s == StreamStatus::Done {
            return s;
        }
        let mut groups = state.recorded_chunk_groups.write();
        debug_assert!(groups.len() >= 1);
        let last_group = groups.last_mut().unwrap();
        if last_group.chunks().len() < last_group.chunk_capacity() {
            last_group.push_chunk(dst);
        } else {
            let mut new_group = SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP);
            new_group.push_chunk(dst);
            groups.push(new_group);
        }
        StreamStatus::Playing
    }

    fn serialize(&self, mut serializer: Serializer) {
        let data = self.data.recorded_chunk_groups.read();
        serializer.array_iter_f32(data.iter().flat_map(|b| b.samples()).flatten());
    }
}

impl WithObjectType for Recorder {
    const TYPE: ObjectType = ObjectType::new("recorder");
}
