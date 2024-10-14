use std::any::Any;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    soundchunk::SoundChunk,
};

use super::{
    context::{Context, LocalArrayList, ProcessorFrameData},
    soundinput::{
        BasicProcessorInput, InputOptions, InputTiming, ProcessorInputId, SoundInputBranchId,
    },
    soundprocessor::{
        CompiledProcessorComponent, ProcessorComponent, ProcessorComponentVisitor,
        ProcessorComponentVisitorMut, SoundProcessorId, StreamStatus,
    },
};

pub struct SingleInput {
    input: BasicProcessorInput,
}

impl SingleInput {
    pub fn new(options: InputOptions) -> SingleInput {
        let branches = vec![Self::THE_ONLY_BRANCH];
        SingleInput {
            input: BasicProcessorInput::new(options, branches),
        }
    }

    pub fn id(&self) -> ProcessorInputId {
        self.input.id()
    }

    const THE_ONLY_BRANCH: SoundInputBranchId = SoundInputBranchId::new(1);
}

impl ProcessorComponent for SingleInput {
    type CompiledType<'ctx> = SingleInputNode<'ctx>;

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
        SingleInputNode::new(self.input.compile(processor_id, compiler))
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
pub struct SingleInputNode<'ctx> {
    target: CompiledSoundInputBranch<'ctx>,
}

impl<'ctx> SingleInputNode<'ctx> {
    fn new<'a>(compiled_input: CompiledSoundInputBranch<'ctx>) -> SingleInputNode<'ctx> {
        SingleInputNode {
            target: compiled_input,
        }
    }

    pub fn timing(&self) -> &InputTiming {
        self.target.timing()
    }

    pub fn timing_mut(&mut self) -> &mut InputTiming {
        self.target.timing_mut()
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

    pub fn start_over(&mut self, sample_offset: usize) {
        self.target.start_over(sample_offset);
    }
}

impl<'ctx> CompiledProcessorComponent<'ctx> for SingleInputNode<'ctx> {
    fn start_over(&mut self) {
        SingleInputNode::start_over(self, 0);
    }
}
