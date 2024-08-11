use crate::core::{
    engine::soundgraphcompiler::SoundGraphCompiler,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::Context,
        soundprocessor::{
            DynamicSoundProcessor, ProcessorTiming, StateAndTiming, StaticSoundProcessor,
            StaticSoundProcessorWithId, StreamStatus,
        },
        soundprocessortools::SoundProcessorTools,
        state::State,
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

    type Expressions<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Self {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn process_audio(
        _processor: &StaticSoundProcessorWithId<TestStaticSoundProcessor>,
        _timing: &ProcessorTiming,
        _sound_inputs: &mut (),
        _expressions: &mut (),
        _dst: &mut SoundChunk,
        _context: Context,
    ) {
    }
}

impl WithObjectType for TestStaticSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("teststatic");
}

pub(super) struct TestDynamicSoundProcessor {}

impl TestDynamicSoundProcessor {
    pub(super) fn new() -> Self {
        Self {}
    }
}

pub(super) struct TestDynamicSoundProcessorStatic {}

impl State for TestDynamicSoundProcessorStatic {
    fn start_over(&mut self) {}
}

impl DynamicSoundProcessor for TestDynamicSoundProcessor {
    type SoundInputType = ();

    type Expressions<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Self {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    type StateType = TestDynamicSoundProcessorStatic;

    fn make_state(&self) -> Self::StateType {
        todo!()
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<TestDynamicSoundProcessorStatic>,
        _sound_inputs: &mut (),
        _expressions: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }
}

impl WithObjectType for TestDynamicSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testdynamic");
}
