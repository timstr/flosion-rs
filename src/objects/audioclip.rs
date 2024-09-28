use std::{ops::Deref, sync::Arc};

use parking_lot::RwLock;

use crate::{
    core::{
        audiofileio::load_audio_file,
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundprocessor::{StateAndTiming, StreamStatus, WhateverSoundProcessor},
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundbuffer::SoundBuffer,
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::{FilePathArgument, ParsedArguments},
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

impl AudioClip {
    pub const ARG_PATH: FilePathArgument = FilePathArgument("path");
}

impl State for AudioClipState {
    fn start_over(&mut self) {
        self.playhead = 0;
    }
}

impl WhateverSoundProcessor for AudioClip {
    type StateType = AudioClipState;
    type SoundInputType = ();
    type Expressions<'ctx> = ();

    fn new(_tools: SoundProcessorTools, args: &ParsedArguments) -> Result<Self, ()> {
        let buffer = if let Some(path) = args.get(&Self::ARG_PATH) {
            if let Ok(b) = load_audio_file(&path) {
                b
            } else {
                println!("Failed to load audio file from \"{}\"", path.display());
                SoundBuffer::new_empty()
            }
        } else {
            SoundBuffer::new_empty()
        };
        Ok(AudioClip {
            data: Arc::new(RwLock::new(buffer)),
        })
    }

    fn is_static(&self) -> bool {
        false
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

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn process_audio(
        state: &mut StateAndTiming<AudioClipState>,
        _sound_inputs: &mut Self::SoundInputType,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        _context: Context,
    ) -> StreamStatus {
        let st = state.state_mut();
        // TODO: avoid locking here? Maybe use ArcSwap
        let data = st.data.read();
        if data.sample_len() == 0 {
            dst.silence();
            return StreamStatus::Done;
        }
        if st.playhead >= data.sample_len() {
            st.playhead = 0;
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
}

impl WithObjectType for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}
