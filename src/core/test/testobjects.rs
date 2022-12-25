use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    soundchunk::SoundChunk,
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
};

pub(super) struct TestStaticSoundProcessor {}

impl TestStaticSoundProcessor {
    pub(super) fn new() -> Self {
        Self {}
    }
}

impl StaticSoundProcessor for TestStaticSoundProcessor {
    type SoundInputType = ();

    type NumberInputType = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Self {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_number_inputs(&self) -> Self::NumberInputType {
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
