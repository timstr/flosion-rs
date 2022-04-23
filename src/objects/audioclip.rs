use parking_lot::RwLock;

use crate::core::{
    context::ProcessorContext,
    graphobject::{ObjectType, TypedGraphObject},
    soundchunk::SoundChunk,
    soundprocessor::DynamicSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    soundstate::SoundState,
};

pub struct AudioClip {
    audio_data: RwLock<Vec<(f32, f32)>>,
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
        todo!()
    }
}

impl TypedGraphObject for AudioClip {
    const TYPE: ObjectType = ObjectType::new("audioclip");
}
