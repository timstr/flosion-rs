use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use chive::ChiveIn;
use parking_lot::RwLock;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            soundinput::InputOptions,
            soundinputtypes::{SingleInput, SingleInputNode},
            soundprocessor::{
                StateAndTiming, StaticSoundProcessor, StaticSoundProcessorWithId, StreamStatus,
            },
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundbuffer::SoundBuffer,
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

const CHUNKS_PER_GROUP: usize = 64;

struct RecorderData {
    // TODO: use a more appropriate data structure
    recorded_chunk_groups: RwLock<Vec<SoundBuffer>>,
    recording: AtomicBool,
}

pub struct Recorder {
    pub input: SingleInput,
    shared_data: Arc<RecorderData>,
}

pub struct RecorderState {
    shared_data: Arc<RecorderData>,
}

impl State for RecorderState {
    fn start_over(&mut self) {
        // ???
    }
}

impl Recorder {
    pub fn start_recording(&self) {
        self.shared_data.recording.store(true, Ordering::Relaxed)
    }

    pub fn stop_recording(&self) {
        self.shared_data.recording.store(false, Ordering::Relaxed);
    }

    pub fn is_recording(&self) -> bool {
        self.shared_data.recording.load(Ordering::Relaxed)
    }

    pub fn copy_audio(&self) -> SoundBuffer {
        let chunk_groups = self.shared_data.recorded_chunk_groups.read();
        let mut b = SoundBuffer::new_with_capacity(chunk_groups.len() * CHUNKS_PER_GROUP);
        for cg in &*chunk_groups {
            for c in cg.chunks() {
                b.push_chunk(c);
            }
        }
        b
    }

    pub fn clear_recording(&self) {
        // TODO: this throws away the existing vector's capacity.
        // Vec::clear() should be used instead, or a more appropriate data structure
        let buf = SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP);
        *self.shared_data.recorded_chunk_groups.write() = vec![buf];
    }

    pub fn recording_length(&self) -> usize {
        let mut n: usize = 0;
        for ch in &*self.shared_data.recorded_chunk_groups.read() {
            n += CHUNK_SIZE * ch.chunks().len();
        }
        n
    }
}

impl StaticSoundProcessor for Recorder {
    type SoundInputType = SingleInput;
    type Expressions<'ctx> = ();
    type StateType = RecorderState;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        let buf = SoundBuffer::new_with_capacity(CHUNKS_PER_GROUP);

        let shared_data = RecorderData {
            recorded_chunk_groups: RwLock::new(vec![buf]),
            recording: AtomicBool::new(false),
        };

        let r = Recorder {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            shared_data: Arc::new(shared_data),
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

    fn make_state(&self) -> Self::StateType {
        RecorderState {
            shared_data: Arc::clone(&self.shared_data),
        }
    }

    fn process_audio(
        // TODO: remove
        _recorder: &StaticSoundProcessorWithId<Recorder>,
        state: &mut StateAndTiming<Self::StateType>,
        sound_inputs: &mut SingleInputNode,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        ctx: Context,
    ) {
        let s = sound_inputs.step(state, dst, &ctx, LocalArrayList::new());
        let recording = state.shared_data.recording.load(Ordering::Relaxed);
        if !recording || s == StreamStatus::Done {
            return;
        }
        let mut groups = state.shared_data.recorded_chunk_groups.write();
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

    fn serialize(&self, mut chive_in: ChiveIn) {
        let data = self.shared_data.recorded_chunk_groups.read();
        chive_in.array_iter_f32(data.iter().flat_map(|b| b.samples()).flatten());
    }
}

impl WithObjectType for Recorder {
    const TYPE: ObjectType = ObjectType::new("recorder");
}
