use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        argument::ArgumentScope,
        soundinput::{
            AnyProcessorInput, BasicProcessorInput, Chronicity, InputContext, InputTiming,
            ProcessorInputId,
        },
        soundprocessor::{
            ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
            SoundProcessorId, StartOver, StreamStatus,
        },
    },
    soundchunk::SoundChunk,
    stashing::{StashingContext, UnstashingContext},
};

pub struct SingleInput {
    input: BasicProcessorInput,
}

impl SingleInput {
    pub fn new(chronicity: Chronicity, scope: ArgumentScope) -> SingleInput {
        SingleInput {
            input: BasicProcessorInput::new(chronicity, 1, scope),
        }
    }

    pub fn id(&self) -> ProcessorInputId {
        self.input.id()
    }
}

impl ProcessorComponent for SingleInput {
    type CompiledType<'ctx> = CompiledSingleInput<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.input(self);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.input(self);
    }

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledSingleInput::new(self.input.compile_branch(processor_id, compiler))
    }
}

impl Stashable<StashingContext> for SingleInput {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
    }
}

impl Unstashable<UnstashingContext<'_>> for SingleInput {
    fn unstash(unstasher: &mut Unstasher<UnstashingContext>) -> Result<SingleInput, UnstashError> {
        Ok(SingleInput {
            input: unstasher.object()?,
        })
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for SingleInput {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)
    }
}

pub struct CompiledSingleInput<'ctx> {
    target: CompiledSoundInputBranch<'ctx>,
}

impl<'ctx> CompiledSingleInput<'ctx> {
    fn new<'a>(compiled_input: CompiledSoundInputBranch<'ctx>) -> CompiledSingleInput<'ctx> {
        CompiledSingleInput {
            target: compiled_input,
        }
    }

    pub fn timing(&self) -> &InputTiming {
        self.target.timing()
    }

    pub fn step(&mut self, dst: &mut SoundChunk, ctx: InputContext) -> StreamStatus {
        self.target.step(dst, ctx)
    }

    pub fn start_over_at(&mut self, sample_offset: usize) {
        self.target.start_over_at(sample_offset);
    }
}

impl<'ctx> StartOver for CompiledSingleInput<'ctx> {
    fn start_over(&mut self) {
        CompiledSingleInput::start_over_at(self, 0);
    }
}

impl AnyProcessorInput for SingleInput {
    fn id(&self) -> ProcessorInputId {
        self.input.id()
    }
}
