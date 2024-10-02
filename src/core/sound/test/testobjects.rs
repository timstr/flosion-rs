use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            expression::ProcessorExpression,
            soundprocessor::{
                SoundProcessorId, StateAndTiming, StreamStatus, WhateverSoundProcessor,
            },
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub(super) struct TestStaticSoundProcessor {}

impl WhateverSoundProcessor for TestStaticSoundProcessor {
    type SoundInputType = ();

    type Expressions<'ctx> = ();

    type StateType = ();

    fn new(_tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(Self {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn visit_expressions<'a>(&self, _f: Box<dyn 'a + FnMut(&ProcessorExpression)>) {}

    fn compile_expressions<'a, 'ctx>(
        &self,
        _processor_id: SoundProcessorId,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn process_audio(
        _state: &mut StateAndTiming<Self::StateType>,
        _sound_inputs: &mut (),
        _expressions: &mut (),
        _dst: &mut SoundChunk,
        _context: Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }

    fn is_static(&self) -> bool {
        true
    }
}

impl WithObjectType for TestStaticSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("teststatic");
}

pub(super) struct TestDynamicSoundProcessor {}

pub(super) struct TestDynamicSoundProcessorStatic {}

impl State for TestDynamicSoundProcessorStatic {
    fn start_over(&mut self) {}
}

impl WhateverSoundProcessor for TestDynamicSoundProcessor {
    type SoundInputType = ();

    type Expressions<'ctx> = ();

    fn new(_tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(Self {})
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn visit_expressions<'a>(&self, _f: Box<dyn 'a + FnMut(&ProcessorExpression)>) {}

    fn compile_expressions<'a, 'ctx>(
        &self,
        _processor_id: SoundProcessorId,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    type StateType = TestDynamicSoundProcessorStatic;

    fn make_state(&self) -> Self::StateType {
        todo!()
    }

    fn process_audio<'ctx>(
        _state: &mut StateAndTiming<TestDynamicSoundProcessorStatic>,
        _sound_inputs: &mut (),
        _expressions: &mut (),
        _dst: &mut SoundChunk,
        _context: Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }

    fn is_static(&self) -> bool {
        false
    }
}

impl WithObjectType for TestDynamicSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testdynamic");
}
