use hashstash::{
    InplaceUnstasher, Stashable, Stasher, UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        argument::ArgumentScope,
        soundinput::{
            Chronicity, InputContext, InputTiming, ProcessorInput, SoundInputBackend,
            SoundInputBranching, SoundInputLocation,
        },
        soundprocessor::{SoundProcessorId, StartOver, StreamStatus},
    },
    soundchunk::SoundChunk,
    stashing::{StashingContext, UnstashingContext},
};

pub struct SingleInputBackend {
    chronicity: Chronicity,
}

impl SingleInputBackend {
    pub fn new(chronicity: Chronicity) -> SingleInputBackend {
        SingleInputBackend { chronicity }
    }
}

impl SoundInputBackend for SingleInputBackend {
    type CompiledType<'ctx> = CompiledSingleInput<'ctx>;

    fn branching(&self) -> SoundInputBranching {
        SoundInputBranching::Unbranched
    }

    fn chronicity(&self) -> Chronicity {
        self.chronicity
    }

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledSingleInput::new(CompiledSoundInputBranch::new(
            location,
            compiler.compile_sound_processor(target),
        ))
    }
}

impl Stashable<StashingContext> for SingleInputBackend {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.chronicity);
    }
}

impl Unstashable<UnstashingContext<'_>> for SingleInputBackend {
    fn unstash(
        unstasher: &mut Unstasher<UnstashingContext>,
    ) -> Result<SingleInputBackend, UnstashError> {
        Ok(SingleInputBackend {
            chronicity: unstasher.object()?,
        })
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for SingleInputBackend {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_replace(&mut self.chronicity)?;
        Ok(())
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

pub type SingleInput = ProcessorInput<SingleInputBackend>;

impl SingleInput {
    pub fn new(chronicity: Chronicity, argument_scope: ArgumentScope) -> SingleInput {
        ProcessorInput::new_from_parts(argument_scope, SingleInputBackend { chronicity })
    }
}
