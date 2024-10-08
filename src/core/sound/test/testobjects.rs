use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundinput::{InputOptions, ProcessorInput, SoundInputBranchId},
            soundprocessor::{
                ProcessorComponentVisitor, ProcessorComponentVisitorMut, SoundProcessorId,
                StreamStatus, WhateverCompiledSoundProcessor, WhateverSoundProcessor,
            },
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub(super) struct TestStaticSoundProcessor {
    pub(super) inputs: Vec<ProcessorInput>,
}

impl TestStaticSoundProcessor {
    pub(super) fn add_input(
        &mut self,
        options: InputOptions,
        branches: Vec<SoundInputBranchId>,
        tools: &mut SoundProcessorTools,
    ) {
        self.inputs.push(tools.make_sound_input(options, branches));
    }
}

pub(super) struct CompiledTestStaticSoundProcessor {}

impl WhateverSoundProcessor for TestStaticSoundProcessor {
    type CompiledType<'ctx> = CompiledTestStaticSoundProcessor;

    fn new(_tools: SoundProcessorTools, _args: &ParsedArguments) -> Self {
        Self { inputs: Vec::new() }
    }

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        for input in &self.inputs {
            visitor.input(input);
        }
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        for input in &mut self.inputs {
            visitor.input(input);
        }
    }

    fn is_static(&self) -> bool {
        true
    }

    fn compile<'ctx>(
        &self,
        _id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledTestStaticSoundProcessor {}
    }
}

impl<'ctx> WhateverCompiledSoundProcessor<'ctx> for CompiledTestStaticSoundProcessor {
    fn process_audio(&mut self, _dst: &mut SoundChunk, _context: Context) -> StreamStatus {
        StreamStatus::Done
    }

    fn start_over(&mut self) {}
}

impl WithObjectType for TestStaticSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("teststatic");
}

pub(super) struct TestDynamicSoundProcessor {}

pub(super) struct CompiledTestDynamicSoundProcessor {}

impl WhateverSoundProcessor for TestDynamicSoundProcessor {
    type CompiledType<'ctx> = CompiledTestDynamicSoundProcessor;

    fn new(_tools: SoundProcessorTools, _args: &ParsedArguments) -> Self {
        Self {}
    }

    fn visit<'a>(&self, _visitor: &'a mut dyn ProcessorComponentVisitor) {}

    fn visit_mut<'a>(&mut self, _visitor: &'a mut dyn ProcessorComponentVisitorMut) {}

    fn is_static(&self) -> bool {
        false
    }

    fn compile<'ctx>(
        &self,
        _id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledTestDynamicSoundProcessor {}
    }
}

impl<'ctx> WhateverCompiledSoundProcessor<'ctx> for CompiledTestDynamicSoundProcessor {
    fn process_audio(&mut self, _dst: &mut SoundChunk, _context: Context) -> StreamStatus {
        StreamStatus::Done
    }

    fn start_over(&mut self) {}
}

impl WithObjectType for TestDynamicSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testdynamic");
}
