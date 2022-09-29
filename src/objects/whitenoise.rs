use rand::prelude::*;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    soundchunk::SoundChunk,
    soundprocessor::{SoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{NoInputs, NoState, ProcessorState},
};

pub struct WhiteNoise {
    inputs: NoInputs,
}

impl SoundProcessor for WhiteNoise {
    const IS_STATIC: bool = false;

    type StateType = NoState;

    type InputType = NoInputs;

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Self {
        WhiteNoise {
            inputs: NoInputs::new(),
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.inputs
    }

    fn make_state(&self) -> Self::StateType {
        NoState {}
    }

    fn process_audio(
        _state: &mut ProcessorState<NoState>,
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
