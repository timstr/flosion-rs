use std::{ops::Deref, sync::Arc};

use parking_lot::RwLock;

use crate::core::{
    graphobject::{ObjectType, WithObjectType},
    serialization::Serializer,
    soundbuffer::SoundBuffer,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::{SoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{NoInputs, ProcessorState, State},
};

pub struct AudioClip {
    data: Arc<RwLock<SoundBuffer>>,
    input: NoInputs,
}

impl AudioClip {
    pub fn set_data(&self, data: SoundBuffer) {
        *self.data.write() = data;
    }

    pub fn get_data<'a>(&'a self) -> impl 'a + Deref<Target = SoundBuffer> {
        self.data.read()
    }
}

pub struct AudioClipState {
    data: Arc<RwLock<SoundBuffer>>,
    playhead: usize,
}

impl State for AudioClipState {
    fn reset(&mut self) {
        self.playhead = 0;
    }
}

impl SoundProcessor for AudioClip {
    const IS_STATIC: bool = false;

    type StateType = AudioClipState;
    type InputType = NoInputs;

    fn new(_tools: SoundProcessorTools) -> Self {
        let data = SoundBuffer::new_empty();
        AudioClip {
            data: Arc::new(RwLock::new(data)),
            input: NoInputs::new(),
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        AudioClipState {
            data: Arc::clone(&self.data),
            playhead: 0,
        }
    }

    fn process_audio(
        state: &mut ProcessorState<AudioClipState>,
        _inputs: &mut Self::InputType,
        dst: &mut SoundChunk,
        _context: crate::core::context::Context,
    ) -> StreamStatus {
        let st = state.state_mut();
        let data = st.data.read();
        if data.sample_len() == 0 {
            dst.silence();
            return StreamStatus::Done;
        }
        for i in 0..CHUNK_SIZE {
            // TODO: don't repeat this every sample
            let ci = st.playhead / CHUNK_SIZE;
            let si = st.playhead % CHUNK_SIZE;
            let c = &data.chunks()[ci];
            st.playhead += 1;
            if st.playhead >= data.sample_len() {
                // TODO: add an option to enable/disable looping
                st.playhead -= data.sample_len();
            }
            debug_assert!(st.playhead < data.sample_len());
            dst.l[i] = c.l[si];
            dst.r[i] = c.r[si];
        }
        StreamStatus::Playing
    }

    fn serialize(&self, serializer: Serializer) {
        let data = self.data.read();
        serializer.array_iter_f32(
            data.chunks()
                .iter()
                .map(|c| &c.l[..])
                .flatten()
                .take(data.sample_len())
                .cloned(),
        );
        serializer.array_iter_f32(
            data.chunks()
                .iter()
                .map(|c| &c.r[..])
                .flatten()
                .take(data.sample_len())
                .cloned(),
        );
    }
}

impl WithObjectType for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}
