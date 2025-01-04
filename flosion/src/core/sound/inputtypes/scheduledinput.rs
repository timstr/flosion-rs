use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        argument::ArgumentScope,
        soundinput::{ProcessorInput, SoundInputBackend, SoundInputCategory, SoundInputLocation},
        soundprocessor::{SoundProcessorId, StartOver},
    },
};

pub struct ScheduledInputBackend {}

impl SoundInputBackend for ScheduledInputBackend {
    type CompiledType<'ctx> = CompiledScheduledInput<'ctx>;

    fn category(&self) -> SoundInputCategory {
        SoundInputCategory::Scheduled
    }

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledScheduledInput {
            target: CompiledSoundInputBranch::new(
                location,
                compiler.compile_sound_processor(target),
            ),
        }
    }
}

pub struct CompiledScheduledInput<'ctx> {
    target: CompiledSoundInputBranch<'ctx>,
}

impl<'ctx> StartOver for CompiledScheduledInput<'ctx> {
    fn start_over(&mut self) {
        todo!()
    }
}

pub type ScheduledInput = ProcessorInput<ScheduledInputBackend>;

impl ScheduledInput {
    pub fn new(argument_scope: ArgumentScope) -> ScheduledInput {
        ProcessorInput::new_from_parts(argument_scope, ScheduledInputBackend {})
    }
}
