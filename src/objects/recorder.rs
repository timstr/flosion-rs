use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::RwLock;
use serialization::{Serializable, Serializer};

use crate::core::{
    engine::soundgraphcompiler::SoundGraphCompiler,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::{Context, LocalArrayList},
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundprocessor::{
            ProcessorTiming, StaticSoundProcessor, StaticSoundProcessorWithId, StreamStatus,
        },
        soundprocessortools::SoundProcessorTools,
    },
    soundbuffer::SoundBuffer,
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

const CHUNKS_PER_GROUP: usize = 64;

pub struct Recorder {
    pub input: SingleInput,
    recorded_chunk_groups: RwLock<Vec<SoundBuffer>>,
    recording: AtomicBool,
}

impl Recorder {
    pub fn start_recording(&self) {
        self.recording.store(true, Ordering::Relaxed)
    }

    pub fn stop_recording(&self) {
        self.recording.store(false, Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }

    pub fn copy_audio(&self) -> SoundBuffer {
        let chunk_groups = self.recorded_chunk_groups.read();
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
        *self.recorded_chunk_groups.write() = vec![buf];
    }

    pub fn recording_length(&self) -> usize {
        let mut n: usize = 0;
        for ch in &*self.recorded_chunk_groups.read() {
            n += CHUNK_SIZE * ch.chunks().len();
        }
        n
    }
}

impl StaticSoundProcessor for Recorder {
    type SoundInputType = SingleInput;
    type Expressions<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let buf = match _init {
            ObjectInitialization::Archive(mut a) => SoundBuffer::deserialize(&mut a)?,
            _ => SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP),
        };
        let r = Recorder {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            recorded_chunk_groups: RwLock::new(vec![buf]),
            recording: AtomicBool::new(false),
        };
        debug_assert!(r.recording_length() == 0);
        Ok(r)
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn process_audio(
        recorder: &StaticSoundProcessorWithId<Recorder>,
        timing: &ProcessorTiming,
        sound_inputs: &mut SingleInputNode,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        ctx: Context,
    ) {
        let s = sound_inputs.step(timing, dst, &ctx, LocalArrayList::new());
        let recording = recorder.recording.load(Ordering::Relaxed);
        if !recording || s == StreamStatus::Done {
            return;
        }
        let mut groups = recorder.recorded_chunk_groups.write();
        debug_assert!(groups.len() >= 1);
        let last_group = groups.last_mut().unwrap();
        if last_group.chunks().len() < last_group.chunk_capacity() {
            last_group.push_chunk(dst);
        } else {
            let mut new_group = SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP);
            new_group.push_chunk(dst);
            groups.push(new_group);
        }
    }

    fn serialize(&self, mut serializer: Serializer) {
        let data = self.recorded_chunk_groups.read();
        serializer.array_iter_f32(data.iter().flat_map(|b| b.samples()).flatten());
    }
}

impl WithObjectType for Recorder {
    const TYPE: ObjectType = ObjectType::new("recorder");
}
