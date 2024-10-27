use hashstash::{InplaceUnstasher, Order, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        expression::expressionobject::ExpressionObjectFactory,
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
        stashing::StashingContext,
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
        processor: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }

    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher,
        factory: &ExpressionObjectFactory,
    ) -> Result<(), UnstashError> {
        todo!()
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

impl Stashable for TestStaticSoundProcessor {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.array_of_objects_slice(&self.inputs, Order::Ordered);
    }
}

impl UnstashableInplace for TestStaticSoundProcessor {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
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
        processor: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        StreamStatus::Done
    }

    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher,
        factory: &ExpressionObjectFactory,
    ) -> Result<(), UnstashError> {
        todo!()
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

impl Stashable for TestDynamicSoundProcessor {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.array_of_objects_slice(&self.inputs, Order::Ordered);
    }
}

impl UnstashableInplace for TestDynamicSoundProcessor {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.array_of_objects_vec_inplace(&mut self.inputs)
    }
}
