use std::rc::Rc;

use inkwell::values::FloatValue;

use crate::core::{
    engine::soundgraphcompiler::SoundGraphCompiler,
    jit::{
        jit::Jit,
        wrappers::{ArrayReadFunc, ScalarReadFunc},
    },
    uniqueid::UniqueId,
};

use super::{
    soundinput::{ProcessorInputId, SoundInputLocation},
    soundprocessor::{
        ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
        SoundProcessorId,
    },
};

pub struct ProcessorArgumentTag;

pub type ProcessorArgumentId = UniqueId<ProcessorArgumentTag>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub(crate) struct ProcessorArgumentLocation {
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ProcessorArgumentDataSource {
    ProcessorState,
    LocalVariable,
}

pub struct ProcessorArgument {
    id: ProcessorArgumentId,
    instance: Rc<dyn AnyProcessorArgument>,
}

impl ProcessorArgument {
    pub(super) fn new(
        id: ProcessorArgumentId,
        instance: Rc<dyn AnyProcessorArgument>,
    ) -> ProcessorArgument {
        ProcessorArgument { id, instance }
    }

    pub(crate) fn id(&self) -> ProcessorArgumentId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn AnyProcessorArgument {
        &*self.instance
    }
}

impl ProcessorComponent for ProcessorArgument {
    type CompiledType<'ctx> = ();

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.processor_argument(self);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.processor_argument(self);
    }

    fn compile<'ctx>(
        &self,
        _processor_id: SoundProcessorId,
        _compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> () {
        ()
    }
}

pub(crate) trait AnyProcessorArgument {
    fn data_source(&self) -> ProcessorArgumentDataSource;

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: ProcessorArgumentLocation,
    ) -> FloatValue<'ctx>;
}

// ----------------------------

pub struct SoundInputArgumentTag;

pub type SoundInputArgumentId = UniqueId<SoundInputArgumentTag>;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub(crate) struct SoundInputArgumentLocation {
    processor: SoundProcessorId,
    input: ProcessorInputId,
    argument: SoundInputArgumentId,
}

impl SoundInputArgumentLocation {
    pub(crate) fn new(
        processor: SoundProcessorId,
        input: ProcessorInputId,
        argument: SoundInputArgumentId,
    ) -> SoundInputArgumentLocation {
        SoundInputArgumentLocation {
            processor,
            input,
            argument,
        }
    }

    pub(crate) fn processor(&self) -> SoundProcessorId {
        self.processor
    }

    pub(crate) fn input(&self) -> ProcessorInputId {
        self.input
    }

    pub(crate) fn argument(&self) -> SoundInputArgumentId {
        self.argument
    }
}

pub struct SoundInputArgument {
    id: SoundInputArgumentId,
    instance: Rc<dyn AnySoundInputArgument>,
}

impl SoundInputArgument {
    pub(super) fn new(
        id: SoundInputArgumentId,
        instance: Rc<dyn AnySoundInputArgument>,
    ) -> SoundInputArgument {
        SoundInputArgument { id, instance }
    }

    pub(crate) fn id(&self) -> SoundInputArgumentId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn AnySoundInputArgument {
        &*self.instance
    }
}

pub(crate) trait AnySoundInputArgument {
    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: SoundInputArgumentLocation,
    ) -> FloatValue<'ctx>;
}

// ----------------------------

#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub(crate) enum ArgumentLocation {
    Processor(ProcessorArgumentLocation),
    Input(SoundInputArgumentLocation),
}

impl From<ProcessorArgumentLocation> for ArgumentLocation {
    fn from(value: ProcessorArgumentLocation) -> Self {
        ArgumentLocation::Processor(value)
    }
}
impl From<&ProcessorArgumentLocation> for ArgumentLocation {
    fn from(value: &ProcessorArgumentLocation) -> Self {
        ArgumentLocation::Processor(*value)
    }
}
impl From<SoundInputArgumentLocation> for ArgumentLocation {
    fn from(value: SoundInputArgumentLocation) -> Self {
        ArgumentLocation::Input(value)
    }
}
impl From<&SoundInputArgumentLocation> for ArgumentLocation {
    fn from(value: &SoundInputArgumentLocation) -> Self {
        ArgumentLocation::Input(*value)
    }
}

// ----------------------------

// A SoundExpressionArgument that reads a scalar from the state of a sound input
pub(crate) struct ScalarInputExpressionArgument {
    function: ScalarReadFunc,
}

impl ScalarInputExpressionArgument {
    pub(super) fn new(function: ScalarReadFunc) -> ScalarInputExpressionArgument {
        ScalarInputExpressionArgument { function }
    }
}

impl AnySoundInputArgument for ScalarInputExpressionArgument {
    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: SoundInputArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_input_scalar_read(
            SoundInputLocation::new(location.processor(), location.input()),
            self.function,
        )
    }
}

// A SoundExpressionArgument that reads an array from the state of a sound input
pub(crate) struct ArrayInputExpressionArgument {
    function: ArrayReadFunc,
}

impl ArrayInputExpressionArgument {
    pub(super) fn new(function: ArrayReadFunc) -> ArrayInputExpressionArgument {
        ArrayInputExpressionArgument { function }
    }
}

impl AnySoundInputArgument for ArrayInputExpressionArgument {
    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: SoundInputArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_input_array_read(
            SoundInputLocation::new(location.processor(), location.input()),
            self.function,
        )
    }
}

// A SoundExpressionArgument that reads a scalar from the state of a sound processor
pub(crate) struct ScalarProcessorExpressionArgument {
    function: ScalarReadFunc,
}

impl ScalarProcessorExpressionArgument {
    pub(super) fn new(function: ScalarReadFunc) -> ScalarProcessorExpressionArgument {
        ScalarProcessorExpressionArgument { function }
    }
}

impl AnyProcessorArgument for ScalarProcessorExpressionArgument {
    fn data_source(&self) -> ProcessorArgumentDataSource {
        ProcessorArgumentDataSource::ProcessorState
    }

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: ProcessorArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_processor_scalar_read(location.processor, self.function)
    }
}

// A SoundExpressionArgument that reads an array from the state of a sound processor
pub(crate) struct ArrayProcessorExpressionArgument {
    function: ArrayReadFunc,
}

impl ArrayProcessorExpressionArgument {
    pub(super) fn new(function: ArrayReadFunc) -> ArrayProcessorExpressionArgument {
        ArrayProcessorExpressionArgument { function }
    }
}

impl AnyProcessorArgument for ArrayProcessorExpressionArgument {
    fn data_source(&self) -> ProcessorArgumentDataSource {
        ProcessorArgumentDataSource::ProcessorState
    }

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: ProcessorArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_processor_array_read(location.processor, self.function)
    }
}

// An ExpressionArgument that evaluates the current time at a sound processor
pub(crate) struct ProcessorTimeExpressionArgument {}

impl ProcessorTimeExpressionArgument {
    pub(super) fn new() -> ProcessorTimeExpressionArgument {
        ProcessorTimeExpressionArgument {}
    }
}

impl AnyProcessorArgument for ProcessorTimeExpressionArgument {
    fn data_source(&self) -> ProcessorArgumentDataSource {
        ProcessorArgumentDataSource::ProcessorState
    }

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: ProcessorArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_processor_time(location.processor)
    }
}

// An ExpressionArgument that evaluates the current time at a sound input
pub(crate) struct InputTimeExpressionArgument {}

impl InputTimeExpressionArgument {
    pub(super) fn new() -> InputTimeExpressionArgument {
        InputTimeExpressionArgument {}
    }
}

impl AnySoundInputArgument for InputTimeExpressionArgument {
    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: SoundInputArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_input_time(SoundInputLocation::new(
            location.processor(),
            location.input(),
        ))
    }
}

// An ExpressionArgument that evaluates an array of data that is local to the
// sound processor's audio callback function
pub(crate) struct ProcessorLocalArrayExpressionArgument {}

impl ProcessorLocalArrayExpressionArgument {
    pub(super) fn new() -> ProcessorLocalArrayExpressionArgument {
        ProcessorLocalArrayExpressionArgument {}
    }
}

impl AnyProcessorArgument for ProcessorLocalArrayExpressionArgument {
    fn data_source(&self) -> ProcessorArgumentDataSource {
        ProcessorArgumentDataSource::LocalVariable
    }

    fn compile<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        location: ProcessorArgumentLocation,
    ) -> FloatValue<'ctx> {
        jit.build_processor_local_array_read(location.processor, location.argument)
    }
}
