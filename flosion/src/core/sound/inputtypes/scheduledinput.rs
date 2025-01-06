use crate::core::{
    engine::{compiledprocessor::CompiledSoundInputNode, soundgraphcompiler::SoundGraphCompiler},
    sound::{
        argument::ArgumentScope,
        soundinput::{ProcessorInput, SoundInputBackend, SoundInputCategory, SoundInputLocation},
        soundprocessor::{
            CompiledComponentVisitor, CompiledProcessorComponent, SoundProcessorId, StartOver,
        },
    },
};

pub struct InputTimeSpan {
    start_sample: usize,
    length_samples: usize,
}

pub struct SoundInputSchedule {
    spans: Vec<InputTimeSpan>,
}

impl SoundInputSchedule {
    fn new() -> SoundInputSchedule {
        SoundInputSchedule { spans: Vec::new() }
    }
}

pub struct ScheduledInputBackend {
    schedule: SoundInputSchedule,
}

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
            node: CompiledSoundInputNode::new(location, compiler.compile_sound_processor(target)),
        }
    }
}

pub struct CompiledScheduledInput<'ctx> {
    node: CompiledSoundInputNode<'ctx>,
}

impl<'ctx> CompiledProcessorComponent for CompiledScheduledInput<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledComponentVisitor) {
        visitor.input_node(&self.node);
    }
}

impl<'ctx> StartOver for CompiledScheduledInput<'ctx> {
    fn start_over(&mut self) {
        todo!()
    }
}

pub type ScheduledInput = ProcessorInput<ScheduledInputBackend>;

impl ScheduledInput {
    pub fn new(argument_scope: ArgumentScope) -> ScheduledInput {
        ProcessorInput::new_from_parts(
            argument_scope,
            ScheduledInputBackend {
                schedule: SoundInputSchedule::new(),
            },
        )
    }
}
