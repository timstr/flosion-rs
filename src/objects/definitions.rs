use crate::core::{
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        soundinputtypes::SingleInput, soundprocessor::DynamicSoundProcessor,
        soundprocessortools::SoundProcessorTools,
    },
};

pub struct Definitions {}

impl DynamicSoundProcessor for Definitions {
    type StateType = ();

    type SoundInputType = SingleInput;

    type NumberInputType<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()> {
        todo!()
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        todo!()
    }

    fn make_state(&self) -> Self::StateType {
        todo!()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &crate::core::engine::nodegen::NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        todo!()
    }

    fn process_audio<'ctx>(
        state: &mut crate::core::sound::soundprocessor::StateAndTiming<Self::StateType>,
        sound_inputs: &mut <Self::SoundInputType as crate::core::engine::soundinputnode::SoundProcessorInput>::NodeType<'ctx>,
        number_inputs: &mut Self::NumberInputType<'ctx>,
        dst: &mut crate::core::soundchunk::SoundChunk,
        context: crate::core::sound::context::Context,
    ) -> crate::core::sound::soundprocessor::StreamStatus {
        todo!()
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}
