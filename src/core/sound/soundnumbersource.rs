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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundNumberSourceId(usize);

impl SoundNumberSourceId {
    pub(crate) fn new(id: usize) -> SoundNumberSourceId {
        SoundNumberSourceId(id)
    }
}

impl Default for SoundNumberSourceId {
    fn default() -> SoundNumberSourceId {
        SoundNumberSourceId(1)
    }
}

impl UniqueId for SoundNumberSourceId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundNumberSourceId {
        SoundNumberSourceId(self.0 + 1)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SoundNumberSourceOwner {
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
}

impl From<SoundNumberSourceOwner> for SoundGraphId {
    fn from(value: SoundNumberSourceOwner) -> Self {
        match value {
            SoundNumberSourceOwner::SoundProcessor(spid) => spid.into(),
            SoundNumberSourceOwner::SoundInput(siid) => siid.into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundNumberSourceOrigin {
    ProcessorState(SoundProcessorId),
    InputState(SoundInputId),
    Local(SoundProcessorId),
}

pub struct SoundNumberSourceHandle {
    id: SoundNumberSourceId,
}

impl SoundNumberSourceHandle {
    pub(super) fn new(id: SoundNumberSourceId) -> SoundNumberSourceHandle {
        SoundNumberSourceHandle { id }
    }

    pub(crate) fn id(&self) -> SoundNumberSourceId {
        self.id
    }
}

// Trait holding the runtime information on how to evaluate/compile a specific
// sound processor or sound input's number source from its state.
// To prevent reference cycles, implementations of this trait should NOT hold
// an Arc to the sound processor or sound input. Rather, any shared data should
// be stored in a separate Arc held by both, and state is always read from the
// Context's state chain during audio processing
pub(crate) trait SoundNumberSource: 'static + Sync + Send {
    // Where does the number source's data come from?
    fn origin(&self) -> SoundNumberSourceOrigin;

    // Produce JIT instructions that evaluate the number source
    // at each sample
    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx>;
}

// A SoundNumberSource that reads a scalar from the state of a sound input
pub(crate) struct ScalarInputNumberSource {
    function: ScalarReadFunc,
    input_id: SoundInputId,
}

impl ScalarInputNumberSource {
    pub(super) fn new(input_id: SoundInputId, function: ScalarReadFunc) -> ScalarInputNumberSource {
        ScalarInputNumberSource { function, input_id }
    }
}

impl SoundNumberSource for ScalarInputNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::InputState(self.input_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_input_scalar_read(self.input_id, self.function)
    }
}

// A SoundNumberSource that reads an array from the state of a sound input
pub(crate) struct ArrayInputNumberSource {
    function: ArrayReadFunc,
    input_id: SoundInputId,
}

impl ArrayInputNumberSource {
    pub(super) fn new(input_id: SoundInputId, function: ArrayReadFunc) -> ArrayInputNumberSource {
        ArrayInputNumberSource { function, input_id }
    }
}

impl SoundNumberSource for ArrayInputNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::InputState(self.input_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_input_array_read(self.input_id, self.function)
    }
}

// A SoundNumberSource that reads a scalar from the state of a sound processor
pub(crate) struct ScalarProcessorNumberSource {
    function: ScalarReadFunc,
    processor_id: SoundProcessorId,
}

impl ScalarProcessorNumberSource {
    pub(super) fn new(
        processor_id: SoundProcessorId,
        function: ScalarReadFunc,
    ) -> ScalarProcessorNumberSource {
        ScalarProcessorNumberSource {
            function,
            processor_id,
        }
    }
}

impl SoundNumberSource for ScalarProcessorNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::ProcessorState(self.processor_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_scalar_read(self.processor_id, self.function)
    }
}

// A SoundNumberSource that reads an array from the state of a sound processor
pub(crate) struct ArrayProcessorNumberSource {
    function: ArrayReadFunc,
    processor_id: SoundProcessorId,
}

impl ArrayProcessorNumberSource {
    pub(super) fn new(
        processor_id: SoundProcessorId,
        function: ArrayReadFunc,
    ) -> ArrayProcessorNumberSource {
        ArrayProcessorNumberSource {
            function,
            processor_id,
        }
    }
}

impl SoundNumberSource for ArrayProcessorNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::ProcessorState(self.processor_id)
    }
    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_array_read(self.processor_id, self.function)
    }
}

// A NumberSource that evaluates the current time at a sound processor
pub(crate) struct ProcessorTimeNumberSource {
    processor_id: SoundProcessorId,
}

impl ProcessorTimeNumberSource {
    pub(super) fn new(processor_id: SoundProcessorId) -> ProcessorTimeNumberSource {
        ProcessorTimeNumberSource { processor_id }
    }
}

impl SoundNumberSource for ProcessorTimeNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::ProcessorState(self.processor_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_time(self.processor_id)
    }
}

// A NumberSource that evaluates the current time at a sound input
pub(crate) struct InputTimeNumberSource {
    input_id: SoundInputId,
}

impl InputTimeNumberSource {
    pub(super) fn new(input_id: SoundInputId) -> InputTimeNumberSource {
        InputTimeNumberSource { input_id }
    }
}

impl SoundNumberSource for InputTimeNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::InputState(self.input_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_input_time(self.input_id)
    }
}

// A NumberSource that evaluates an array of data that is local to the
// sound processor's audio callback function
pub(crate) struct ProcessorLocalArrayNumberSource {
    id: SoundNumberSourceId,
    processor_id: SoundProcessorId,
}

impl ProcessorLocalArrayNumberSource {
    pub(super) fn new(
        id: SoundNumberSourceId,
        processor_id: SoundProcessorId,
    ) -> ProcessorLocalArrayNumberSource {
        ProcessorLocalArrayNumberSource { id, processor_id }
    }
}

impl SoundNumberSource for ProcessorLocalArrayNumberSource {
    fn origin(&self) -> SoundNumberSourceOrigin {
        SoundNumberSourceOrigin::Local(self.processor_id)
    }

    fn compile<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> FloatValue<'ctx> {
        codegen.build_processor_local_array_read(self.processor_id, self.id)
    }
}
