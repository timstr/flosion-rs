use std::{ops::Deref, sync::Arc};

use parking_lot::RwLock;
use serialization::Serializer;

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::Context,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundbuffer::SoundBuffer,
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

pub struct AudioClip {
    data: Arc<RwLock<SoundBuffer>>,
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

impl DynamicSoundProcessor for AudioClip {
    type StateType = AudioClipState;
    type SoundInputType = ();
    type NumberInputType<'ctx> = ();

    fn new(_tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()> {
        let data = match init {
            ObjectInitialization::Archive(mut a) => {
                let mut b = SoundBuffer::new_empty();
                let l = a.peek_length()?;
                if l % 2 != 0 {
                    return Err(());
                }
                b.reserve_chunks(l / (2 * CHUNK_SIZE));
                let mut samples = a.array_iter_f32()?;
                while let Some(l) = samples.next() {
                    let r = samples.next().unwrap();
                    b.push_sample(l, r);
                }
                b
            }
            _ => SoundBuffer::new_empty(),
        };
        Ok(AudioClip {
            data: Arc::new(RwLock::new(data)),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        AudioClipState {
            data: Arc::clone(&self.data),
            playhead: 0,
        }
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio(
        state: &mut StateAndTiming<AudioClipState>,
        _sound_inputs: &mut Self::SoundInputType,
        _number_inputs: &(),
        dst: &mut SoundChunk,
        _context: Context,
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
                st.playhead = 0;
            }
            debug_assert!(st.playhead < data.sample_len());
            dst.l[i] = c.l[si];
            dst.r[i] = c.r[si];
        }
        StreamStatus::Playing
    }

    fn serialize(&self, mut serializer: Serializer) {
        let data = self.data.read();
        serializer.array_iter_f32(data.samples().flatten());
    }
}

impl WithObjectType for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}
