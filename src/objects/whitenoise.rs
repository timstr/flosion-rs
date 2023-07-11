use rand::prelude::*;

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::Context,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub struct WhiteNoise {}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = ();
    type SoundInputType = ();
    type NumberInputType<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WhiteNoise {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio(
        _state: &mut StateAndTiming<()>,
        _sound_inputs: &mut (),
        _number_inputs: &(),
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
