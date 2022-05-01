use parking_lot::RwLock;

use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, TypedGraphObject},
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundprocessor::DynamicSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
};

pub struct AudioClip {
    audio_data: RwLock<Vec<(f32, f32)>>,
}

impl AudioClip {
    pub fn new() -> AudioClip {
        AudioClip {audio_data: RwLock::new(Vec::new())}
    }

    pub fn set_data(&self, data: Vec<(f32, f32)>) {
        *self.audio_data.write() = data;
    }
}

pub struct AudioClipState {
    playhead: usize,
}

impl Default for AudioClipState {
    fn default() -> Self {
        AudioClipState { playhead: 0 }
    }
}

impl SoundState for AudioClipState {
    fn reset(&mut self) {
        self.playhead = 0;
    }
}

impl DynamicSoundProcessor for AudioClip {
    type StateType = AudioClipState;

    fn new(tools: &mut SoundProcessorTools<'_, AudioClipState>) -> AudioClip {
        AudioClip {
            audio_data: RwLock::new(Vec::new()),
        }
    }

    fn process_audio(&self, dst: &mut SoundChunk, context: ProcessorContext<'_, AudioClipState>) {
        let mut state = context.write_state();
        let data = self.audio_data.read();
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

impl TypedGraphObject for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}
