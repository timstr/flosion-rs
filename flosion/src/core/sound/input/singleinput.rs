use std::any::Any;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        context::{Context, LocalArrayList, ProcessorFrameData},
        soundinput::{BasicProcessorInput, InputOptions, InputTiming, ProcessorInputId},
        soundprocessor::{
            ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
            SoundProcessorId, StartOver, StreamStatus,
        },
    },
    soundchunk::SoundChunk,
};

pub struct SingleInput {
    input: BasicProcessorInput,
}

impl SingleInput {
    pub fn new(options: InputOptions) -> SingleInput {
        SingleInput {
            input: BasicProcessorInput::new(options, 1),
        }
    }

    pub fn id(&self) -> ProcessorInputId {
        self.input.id()
    }
}

impl ProcessorComponent for SingleInput {
    type CompiledType<'ctx> = CompiledSingleInput<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.input(&self.input);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.input(&mut self.input);
    }

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledSingleInput::new(self.input.compile_branch(processor_id, compiler))
    }
}

impl Stashable for SingleInput {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.input);
    }
}

impl UnstashableInplace for SingleInput {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)
    }
}

// TODO: rename to CompiledSingleInput
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

    pub fn step(
        &mut self,
        dst: &mut SoundChunk,
        processor_state: Option<&dyn Any>,
        local_arrays: LocalArrayList,
        ctx: &mut Context,
    ) -> StreamStatus {
        self.target.step(
            dst,
            &(),
            ProcessorFrameData::new(processor_state, local_arrays),
            ctx,
        )
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
