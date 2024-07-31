use inkwell::values::FloatValue;

use crate::core::{
    jit::{
        codegen::CodeGen,
        wrappers::{ArrayReadFunc, ScalarReadFunc},
    },
    uniqueid::UniqueId,
};

use super::{
    soundgraphid::SoundGraphId, soundinput::SoundInputId, soundprocessor::SoundProcessorId,
};

pub struct SoundExpressionArgumentTag;

pub type SoundExpressionArgumentId = UniqueId<SoundExpressionArgumentTag>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SoundExpressionArgumentOwner {
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
}

impl From<SoundExpressionArgumentOwner> for SoundGraphId {
    fn from(value: SoundExpressionArgumentOwner) -> Self {
        match value {
            SoundExpressionArgumentOwner::SoundProcessor(spid) => spid.into(),
            SoundExpressionArgumentOwner::SoundInput(siid) => siid.into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundExpressionArgumentOrigin {
    ProcessorState(SoundProcessorId),
    InputState(SoundInputId),
    Local(SoundProcessorId),
}

pub struct SoundExpressionArgumentHandle {
    id: SoundExpressionArgumentId,
}

impl SoundExpressionArgumentHandle {
    pub(super) fn new(id: SoundExpressionArgumentId) -> SoundExpressionArgumentHandle {
        SoundExpressionArgumentHandle { id }
    }

    pub(crate) fn id(&self) -> SoundExpressionArgumentId {
        self.id
    }
}

// Trait holding the runtime information on how to evaluate/compile a specific
// sound processor or sound input's expression argument from its state.
// To prevent reference cycles, implementations of this trait should NOT hold
// an Arc to the sound processor or sound input. Rather, any shared data should
// be stored in a separate Arc held by both, and state is always read from the
// Context's state chain during audio processing
pub(crate) trait SoundExpressionArgument: 'static + Sync + Send {
    // Where does the argument's data come from?
    fn origin(&self) -> SoundExpressionArgumentOrigin;

    // Produce JIT instructions that evaluate the argument
    // at each sample
    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx>;
}

// A SoundExpressionArgument that reads a scalar from the state of a sound input
pub(crate) struct ScalarInputExpressionArgument {
    function: ScalarReadFunc,
    input_id: SoundInputId,
}

impl ScalarInputExpressionArgument {
    pub(super) fn new(
        input_id: SoundInputId,
        function: ScalarReadFunc,
    ) -> ScalarInputExpressionArgument {
        ScalarInputExpressionArgument { function, input_id }
    }
}

impl SoundExpressionArgument for ScalarInputExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::InputState(self.input_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_input_scalar_read(self.input_id, self.function)
    }
}

// A SoundExpressionArgument that reads an array from the state of a sound input
pub(crate) struct ArrayInputExpressionArgument {
    function: ArrayReadFunc,
    input_id: SoundInputId,
}

impl ArrayInputExpressionArgument {
    pub(super) fn new(
        input_id: SoundInputId,
        function: ArrayReadFunc,
    ) -> ArrayInputExpressionArgument {
        ArrayInputExpressionArgument { function, input_id }
    }
}

impl SoundExpressionArgument for ArrayInputExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::InputState(self.input_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_input_array_read(self.input_id, self.function)
    }
}

// A SoundExpressionArgument that reads a scalar from the state of a sound processor
pub(crate) struct ScalarProcessorExpressionArgument {
    function: ScalarReadFunc,
    processor_id: SoundProcessorId,
}

impl ScalarProcessorExpressionArgument {
    pub(super) fn new(
        processor_id: SoundProcessorId,
        function: ScalarReadFunc,
    ) -> ScalarProcessorExpressionArgument {
        ScalarProcessorExpressionArgument {
            function,
            processor_id,
        }
    }
}

impl SoundExpressionArgument for ScalarProcessorExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::ProcessorState(self.processor_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_scalar_read(self.processor_id, self.function)
    }
}

// A SoundExpressionArgument that reads an array from the state of a sound processor
pub(crate) struct ArrayProcessorExpressionArgument {
    function: ArrayReadFunc,
    processor_id: SoundProcessorId,
}

impl ArrayProcessorExpressionArgument {
    pub(super) fn new(
        processor_id: SoundProcessorId,
        function: ArrayReadFunc,
    ) -> ArrayProcessorExpressionArgument {
        ArrayProcessorExpressionArgument {
            function,
            processor_id,
        }
    }
}

impl SoundExpressionArgument for ArrayProcessorExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::ProcessorState(self.processor_id)
    }
    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_array_read(self.processor_id, self.function)
    }
}

// An ExpressionArgument that evaluates the current time at a sound processor
pub(crate) struct ProcessorTimeExpressionArgument {
    processor_id: SoundProcessorId,
}

impl ProcessorTimeExpressionArgument {
    pub(super) fn new(processor_id: SoundProcessorId) -> ProcessorTimeExpressionArgument {
        ProcessorTimeExpressionArgument { processor_id }
    }
}

impl SoundExpressionArgument for ProcessorTimeExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::ProcessorState(self.processor_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_time(self.processor_id)
    }
}

// An ExpressionArgument that evaluates the current time at a sound input
pub(crate) struct InputTimeExpressionArgument {
    input_id: SoundInputId,
}

impl InputTimeExpressionArgument {
    pub(super) fn new(input_id: SoundInputId) -> InputTimeExpressionArgument {
        InputTimeExpressionArgument { input_id }
    }
}

impl SoundExpressionArgument for InputTimeExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::InputState(self.input_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_input_time(self.input_id)
    }
}

// An ExpressionArgument that evaluates an array of data that is local to the
// sound processor's audio callback function
pub(crate) struct ProcessorLocalArrayExpressionArgument {
    id: SoundExpressionArgumentId,
    processor_id: SoundProcessorId,
}

impl ProcessorLocalArrayExpressionArgument {
    pub(super) fn new(
        id: SoundExpressionArgumentId,
        processor_id: SoundProcessorId,
    ) -> ProcessorLocalArrayExpressionArgument {
        ProcessorLocalArrayExpressionArgument { id, processor_id }
    }
}

impl SoundExpressionArgument for ProcessorLocalArrayExpressionArgument {
    fn origin(&self) -> SoundExpressionArgumentOrigin {
        SoundExpressionArgumentOrigin::Local(self.processor_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_local_array_read(self.processor_id, self.id)
    }
}
