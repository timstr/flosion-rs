use std::sync::Arc;

use parking_lot::RwLock;

use crate::core::{
    graphobject::{ObjectType, WithObjectType},
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::SoundProcessor,
    soundprocessortools::SoundProcessorTools,
    statetree::{NoInputs, State},
};

pub struct AudioClip {
    audio_data: Arc<RwLock<Vec<(f32, f32)>>>,
    input: NoInputs,
}

impl AudioClip {
    pub fn set_data(&self, data: Vec<(f32, f32)>) {
        *self.audio_data.write() = data;
    }
}

pub struct AudioClipState {
    audio_data: Arc<RwLock<Vec<(f32, f32)>>>,
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
        AudioClip {
            audio_data: Arc::new(RwLock::new(Vec::new())),
            input: NoInputs::new(),
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        AudioClipState {
            audio_data: Arc::clone(&&self.audio_data),
            playhead: 0,
        }
    }

    fn process_audio(
        state: &mut Self::StateType,
        _inputs: &mut Self::InputType,
        dst: &mut SoundChunk,
        _context: crate::core::context::Context,
    ) {
        let data = state.audio_data.read();
        if data.len() == 0 {
            dst.silence();
            return;
        }
        for i in 0..CHUNK_SIZE {
            let s = data[state.playhead];
            state.playhead += 1;
            if state.playhead >= data.len() {
                state.playhead -= data.len();
            }
            debug_assert!(state.playhead < data.len());
            dst.l[i] = s.0;
            dst.r[i] = s.1;
        }
    }
}

impl WithObjectType for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}
