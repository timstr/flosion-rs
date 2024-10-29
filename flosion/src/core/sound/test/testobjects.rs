use hashstash::{InplaceUnstasher, Order, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundinput::BasicProcessorInput,
            soundprocessor::{
                ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                SoundProcessor, SoundProcessorId, StartOver, StreamStatus,
            },
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

pub(super) struct TestStaticSoundProcessor {
    pub(super) inputs: Vec<BasicProcessorInput>,
}

pub(super) struct CompiledTestStaticSoundProcessor {}

impl SoundProcessor for TestStaticSoundProcessor {
    fn new(_args: &ParsedArguments) -> Self {
        Self { inputs: Vec::new() }
    }

    fn is_static(&self) -> bool {
        true
    }

    fn process_audio(
        _processor: &mut Self::CompiledType<'_>,
        _dst: &mut SoundChunk,
        _context: &mut Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }
}

impl ProcessorComponent for TestStaticSoundProcessor {
    type CompiledType<'ctx> = CompiledTestStaticSoundProcessor;

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

    fn compile<'ctx>(
        &self,
        _id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledTestStaticSoundProcessor {}
    }
}

impl<'ctx> StartOver for CompiledTestStaticSoundProcessor {
    fn start_over(&mut self) {}
}

impl WithObjectType for TestStaticSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("teststatic");
}

impl Stashable<StashingContext> for TestStaticSoundProcessor {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.array_of_objects_slice(&self.inputs, Order::Ordered);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for TestStaticSoundProcessor {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.array_of_objects_vec_inplace(&mut self.inputs)
    }
}

pub(super) struct TestDynamicSoundProcessor {
    pub(super) inputs: Vec<BasicProcessorInput>,
}

pub(super) struct CompiledTestDynamicSoundProcessor {}

impl SoundProcessor for TestDynamicSoundProcessor {
    fn new(_args: &ParsedArguments) -> Self {
        Self { inputs: Vec::new() }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        _processor: &mut Self::CompiledType<'_>,
        _dst: &mut SoundChunk,
        _context: &mut Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }
}

impl ProcessorComponent for TestDynamicSoundProcessor {
    type CompiledType<'ctx> = CompiledTestDynamicSoundProcessor;

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

    fn compile<'ctx>(
        &self,
        _id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledTestDynamicSoundProcessor {}
    }
}

impl StartOver for CompiledTestDynamicSoundProcessor {
    fn start_over(&mut self) {}
}

impl WithObjectType for TestDynamicSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testdynamic");
}

impl Stashable<StashingContext> for TestDynamicSoundProcessor {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.array_of_objects_slice(&self.inputs, Order::Ordered);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for TestDynamicSoundProcessor {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.array_of_objects_vec_inplace(&mut self.inputs)
    }
}
