use hashstash::{
    InplaceUnstasher, Order, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace,
    Unstasher,
};

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ArgumentScope,
            context::AudioContext,
            soundinput::{
                Chronicity, ProcessorInput, SoundInputBackend, SoundInputBranching,
                SoundInputLocation,
            },
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
    pub(super) inputs: Vec<TestSoundInput>,
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
        _context: &mut AudioContext,
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
    pub(super) inputs: Vec<TestSoundInput>,
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
        _context: &mut AudioContext,
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

#[derive(Eq, PartialEq, Debug)]
pub(super) struct TestSoundInputBackend {
    chronicity: Chronicity,
    branching: SoundInputBranching,
}

impl SoundInputBackend for TestSoundInputBackend {
    type CompiledType<'ctx> = ();

    fn branching(&self) -> SoundInputBranching {
        self.branching
    }

    fn chronicity(&self) -> Chronicity {
        self.chronicity
    }

    fn compile<'ctx>(
        &self,
        _location: SoundInputLocation,
        _target: Option<SoundProcessorId>,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        ()
    }
}

impl Stashable<StashingContext> for TestSoundInputBackend {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        self.chronicity.stash(stasher);
        self.branching.stash(stasher);
    }
}

impl<'a> Unstashable<UnstashingContext<'a>> for TestSoundInputBackend {
    fn unstash(unstasher: &mut Unstasher<UnstashingContext>) -> Result<Self, UnstashError> {
        Ok(TestSoundInputBackend {
            chronicity: Unstashable::unstash(unstasher)?,
            branching: Unstashable::unstash(unstasher)?,
        })
    }
}

pub(super) type TestSoundInput = ProcessorInput<TestSoundInputBackend>;

impl TestSoundInput {
    pub(super) fn new(
        chronicity: Chronicity,
        branching: SoundInputBranching,
        argument_scope: ArgumentScope,
    ) -> TestSoundInput {
        ProcessorInput::new_from_parts(
            argument_scope,
            TestSoundInputBackend {
                chronicity,
                branching,
            },
        )
    }
}
