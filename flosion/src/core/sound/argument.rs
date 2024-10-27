use std::marker::PhantomData;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use inkwell::values::FloatValue;

use crate::core::{
    engine::soundgraphcompiler::SoundGraphCompiler,
    jit::{argumentstack::JitArgumentPack, jit::Jit},
    stashing::StashingContext,
    uniqueid::UniqueId,
};

use super::soundprocessor::{
    ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut, SoundProcessorId,
    StartOver,
};

pub struct ProcessorArgumentTag;

pub type ProcessorArgumentId = UniqueId<ProcessorArgumentTag>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct ProcessorArgumentLocation {
    processor: SoundProcessorId,
    argument: ProcessorArgumentId,
}

impl ProcessorArgumentLocation {
    pub(crate) fn new(
        processor: SoundProcessorId,
        argument: ProcessorArgumentId,
    ) -> ProcessorArgumentLocation {
        ProcessorArgumentLocation {
            processor,
            argument,
        }
    }

    pub(crate) fn processor(&self) -> SoundProcessorId {
        self.processor
    }

    pub(crate) fn argument(&self) -> ProcessorArgumentId {
        self.argument
    }
}

pub trait ArgumentTranslation {
    type PushedType<'a>;
    type InternalType: JitArgumentPack;

    fn convert_value(pushed: Self::PushedType<'_>) -> Self::InternalType;

    fn compile<'ctx>(
        values: <Self::InternalType as JitArgumentPack>::InkwellValues<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> FloatValue<'ctx>;
}

pub struct ProcessorArgument<T> {
    id: ProcessorArgumentId,
    code_generator: PhantomData<T>,
}

impl<T: ArgumentTranslation> ProcessorArgument<T> {
    pub fn new() -> ProcessorArgument<T> {
        ProcessorArgument {
            id: ProcessorArgumentId::new_unique(),
            code_generator: PhantomData,
        }
    }

    pub(crate) fn id(&self) -> ProcessorArgumentId {
        self.id
    }

    pub(crate) fn compile_evaluation<'ctx>(&self, jit: &mut Jit<'ctx>) -> FloatValue<'ctx> {
        let ptr = jit.build_argument_pointer(self.id);
        let loaded_values = T::InternalType::generate_load_calls(ptr, jit);
        jit.builder()
            .position_before(&jit.instruction_locations.end_of_loop);
        T::compile(loaded_values, jit)
    }
}

impl<T: ArgumentTranslation + Send> ProcessorComponent for ProcessorArgument<T> {
    type CompiledType<'ctx> = CompiledProcessorArgument<T>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.argument(self);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.argument(self);
    }

    fn compile<'ctx>(
        &self,
        _processor_id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledProcessorArgument<T> {
        CompiledProcessorArgument {
            id: self.id,
            code_generator: PhantomData,
        }
    }
}

impl<T> Stashable for ProcessorArgument<T> {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.id.value() as _);
    }
}

impl<T> UnstashableInplace for ProcessorArgument<T> {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        let id = unstasher.u64_always()?;
        if unstasher.time_to_write() {
            self.id = ProcessorArgumentId::new(id as _);
        }
        Ok(())
    }
}

pub struct CompiledProcessorArgument<T> {
    id: ProcessorArgumentId,
    code_generator: PhantomData<T>,
}

// Implementing Copy and Clone explicitly because
// #[derive(...)] would add incorrect trait bounds
// on T, which is not actually being stored here.
impl<T> Copy for CompiledProcessorArgument<T> {}
impl<T> Clone for CompiledProcessorArgument<T> {
    fn clone(&self) -> CompiledProcessorArgument<T> {
        CompiledProcessorArgument {
            id: self.id,
            code_generator: PhantomData,
        }
    }
}

impl<T> CompiledProcessorArgument<T> {
    pub(crate) fn id(&self) -> ProcessorArgumentId {
        self.id
    }
}

impl<T> StartOver for CompiledProcessorArgument<T> {
    fn start_over(&mut self) {}
}

pub trait AnyProcessorArgument {
    fn id(&self) -> ProcessorArgumentId;

    fn compile_evaluation<'ctx>(&self, jit: &mut Jit<'ctx>) -> FloatValue<'ctx>;
}

impl<T: ArgumentTranslation> AnyProcessorArgument for ProcessorArgument<T> {
    fn id(&self) -> ProcessorArgumentId {
        self.id
    }

    fn compile_evaluation<'ctx>(&self, jit: &mut Jit<'ctx>) -> FloatValue<'ctx> {
        ProcessorArgument::compile_evaluation(self, jit)
    }
}
