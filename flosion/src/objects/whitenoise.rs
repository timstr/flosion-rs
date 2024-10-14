use rand::prelude::*;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub struct WhiteNoise {}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = ();
    type SoundInputType = ();
    type Expressions<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(WhiteNoise {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn process_audio(
        _state: &mut StateAndTiming<()>,
        _sound_inputs: &mut (),
        _expressions: &mut (),
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
