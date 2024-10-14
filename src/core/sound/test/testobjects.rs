use hashstash::{InplaceUnstasher, Order, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            soundinput::BasicProcessorInput,
            soundprocessor::{
                CompiledSoundProcessor, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                SoundProcessor, SoundProcessorId, StreamStatus,
            },
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub(super) struct TestStaticSoundProcessor {
    pub(super) inputs: Vec<BasicProcessorInput>,
}

pub(super) struct CompiledTestStaticSoundProcessor {}

impl SoundProcessor for TestStaticSoundProcessor {
    type CompiledType<'ctx> = CompiledTestStaticSoundProcessor;

    fn new(_args: &ParsedArguments) -> Self {
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

impl<'ctx> CompiledSoundProcessor<'ctx> for CompiledTestStaticSoundProcessor {
    fn process_audio(&mut self, _dst: &mut SoundChunk, _context: Context) -> StreamStatus {
        StreamStatus::Done
    }

    fn start_over(&mut self) {}
}

impl WithObjectType for TestStaticSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("teststatic");
}

impl Stashable for TestStaticSoundProcessor {
    fn stash(&self, stasher: &mut Stasher) {
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
    type CompiledType<'ctx> = CompiledTestDynamicSoundProcessor;

    fn new(_args: &ParsedArguments) -> Self {
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

impl<'ctx> CompiledSoundProcessor<'ctx> for CompiledTestDynamicSoundProcessor {
    fn process_audio(&mut self, _dst: &mut SoundChunk, _context: Context) -> StreamStatus {
        StreamStatus::Done
    }

    fn start_over(&mut self) {}
}

impl WithObjectType for TestDynamicSoundProcessor {
    const TYPE: ObjectType = ObjectType::new("testdynamic");
}

impl Stashable for TestDynamicSoundProcessor {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_objects_slice(&self.inputs, Order::Ordered);
    }
}

impl UnstashableInplace for TestDynamicSoundProcessor {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.array_of_objects_vec_inplace(&mut self.inputs)
    }
}
