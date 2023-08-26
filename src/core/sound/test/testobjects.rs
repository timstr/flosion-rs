use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::Context, soundprocessor::StaticSoundProcessor,
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub(super) struct TestStaticSoundProcessor {}

impl TestStaticSoundProcessor {
    pub(super) fn new() -> Self {
        Self {}
    }
}

impl StaticSoundProcessor for TestStaticSoundProcessor {
    type SoundInputType = ();

    type NumberInputType<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Self {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio(
        &self,
        _sound_inputs: &mut (),
        _number_inputs: &(),
        _dst: &mut SoundChunk,
        _context: Context,
    ) {
    }
}

impl WithObjectType for TestStaticSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("teststaticsoundprocessor");
}
