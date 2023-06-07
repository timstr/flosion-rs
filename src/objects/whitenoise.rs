use rand::prelude::*;

use crate::core::{
    sound::{
        context::Context,
        graphobject::{ObjectInitialization, ObjectType, WithObjectType},
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

    fn make_number_inputs<'ctx>(
        &self,
        _context: &'ctx inkwell::context::Context,
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
