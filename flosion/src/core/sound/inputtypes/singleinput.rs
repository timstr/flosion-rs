use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, compiledprocessor::CompiledSoundInputNode},
    sound::{
        argument::ArgumentScope,
        soundinput::{
            InputContext, InputTiming, ProcessorInput, SoundInputBackend, SoundInputCategory,
            SoundInputLocation,
        },
        soundprocessor::{SoundProcessorId, StartOver, StreamStatus},
    },
    soundchunk::SoundChunk,
    stashing::{StashingContext, UnstashingContext},
};

pub struct SingleInputBackend {
    isochronic: bool,
}

impl SoundInputBackend for SingleInputBackend {
    type CompiledType<'ctx> = CompiledSingleInput<'ctx>;

    fn category(&self) -> SoundInputCategory {
        SoundInputCategory::Anisochronic
    }

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledSingleInput::new(CompiledSoundInputNode::new(
            location,
            compiler.compile_sound_processor(target),
        ))
    }
}

impl Stashable<StashingContext> for SingleInputBackend {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.bool(self.isochronic);
    }
}

impl Unstashable<UnstashingContext<'_>> for SingleInputBackend {
    fn unstash(
        unstasher: &mut Unstasher<UnstashingContext>,
    ) -> Result<SingleInputBackend, UnstashError> {
        Ok(SingleInputBackend {
            isochronic: unstasher.bool()?,
        })
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for SingleInputBackend {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.bool_inplace(&mut self.isochronic)?;
        Ok(())
    }
}

pub struct CompiledSingleInput<'ctx> {
    target: CompiledSoundInputNode<'ctx>,
}

impl<'ctx> CompiledSingleInput<'ctx> {
    fn new<'a>(compiled_input: CompiledSoundInputNode<'ctx>) -> CompiledSingleInput<'ctx> {
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

// TODO: consider splitting into two separate types by chronicity.
// Runtime debug checks can be added to ensure that an isochronic
// single input is always being invoked
pub type SingleInput = ProcessorInput<SingleInputBackend>;

impl SingleInput {
    pub fn new_isochronic(argument_scope: ArgumentScope) -> SingleInput {
        ProcessorInput::new_from_parts(argument_scope, SingleInputBackend { isochronic: true })
    }

    pub fn new_anisochronic(argument_scope: ArgumentScope) -> SingleInput {
        ProcessorInput::new_from_parts(argument_scope, SingleInputBackend { isochronic: false })
    }
}
