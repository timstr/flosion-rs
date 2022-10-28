use rand::prelude::*;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    soundchunk::SoundChunk,
    soundinputtypes::NoInputs,
    soundprocessor::{DynamicSoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::StateAndTiming,
};

pub struct WhiteNoise {
    inputs: NoInputs,
}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = ();

    type InputType = NoInputs;

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WhiteNoise {
            inputs: NoInputs::new(),
        })
    }

    fn get_input(&self) -> &Self::InputType {
        &self.inputs
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn process_audio(
        _state: &mut StateAndTiming<()>,
        _inputs: &mut NoInputs,
        dst: &mut SoundChunk,
        _ctx: Context,
    ) -> StreamStatus {
        for s in dst.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        for s in dst.r.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        StreamStatus::Playing
    }
}

impl WithObjectType for WhiteNoise {
    const TYPE: ObjectType = ObjectType::new("whitenoise");
}
