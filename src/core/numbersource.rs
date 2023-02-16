use std::{ops::Deref, sync::Arc};

use inkwell::values::FloatValue;

use super::{
    compilednumberinput::{ArrayReadFunc, CodeGen, ScalarReadFunc},
    context::Context,
    graphobject::{GraphObjectHandle, ObjectInitialization, WithObjectType},
    numbersourcetools::NumberSourceTools,
    numeric,
    serialization::Serializer,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    state::StateOwner,
    uniqueid::UniqueId,
};

// TODO: consider making usize field private, prefer .value() over .0
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberSourceId(pub usize);

impl Default for NumberSourceId {
    fn default() -> NumberSourceId {
        NumberSourceId(1)
    }
}

impl UniqueId for NumberSourceId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> NumberSourceId {
        NumberSourceId(self.0 + 1)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum NumberSourceOwner {
    Nothing,
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
}

impl NumberSourceOwner {
    pub(super) fn is_stateful(&self) -> bool {
        match self {
            NumberSourceOwner::Nothing => false,
            NumberSourceOwner::SoundProcessor(_) => true,
            NumberSourceOwner::SoundInput(_) => true,
        }
    }

    pub(super) fn as_state_owner(&self) -> Option<StateOwner> {
        match self {
            NumberSourceOwner::Nothing => None,
            NumberSourceOwner::SoundProcessor(spid) => Some(StateOwner::SoundProcessor(*spid)),
            NumberSourceOwner::SoundInput(siid) => Some(StateOwner::SoundInput(*siid)),
        }
    }
}

#[derive(Copy, Clone)]
pub struct NumberConfig {
    samplewise_temporal: bool,
    sample_offset: usize,
}

impl NumberConfig {
    pub fn samplewise_temporal_at(sample_offset: usize) -> NumberConfig {
        NumberConfig {
            samplewise_temporal: true,
            sample_offset,
        }
    }

    pub fn atemporal_at(sample_offset: usize) -> NumberConfig {
        NumberConfig {
            samplewise_temporal: false,
            sample_offset,
        }
    }

    pub fn is_samplewise_temporal(&self) -> bool {
        self.samplewise_temporal
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }
}

pub(crate) trait NumberSource: 'static + Sync + Send {
    fn eval(&self, dst: &mut [f32], context: &Context);

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        _inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn as_graph_object(self: Arc<Self>) -> Option<GraphObjectHandle> {
        None
    }
}

pub trait PureNumberSource: 'static + Sync + Send + WithObjectType {
    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn eval(&self, dst: &mut [f32], context: &Context);

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        _inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn serialize(&self, _serializer: Serializer) {}
}

pub struct PureNumberSourceWithId<T: PureNumberSource> {
    source: T,
    id: NumberSourceId,
}

impl<T: PureNumberSource> PureNumberSourceWithId<T> {
    pub(crate) fn new(source: T, id: NumberSourceId) -> PureNumberSourceWithId<T> {
        PureNumberSourceWithId { source, id }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }
}

impl<T: PureNumberSource> Deref for PureNumberSourceWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.source
    }
}

impl<T: PureNumberSource> NumberSource for PureNumberSourceWithId<T> {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        T::eval(&*self, dst, context)
    }

    fn as_graph_object(self: Arc<Self>) -> Option<GraphObjectHandle> {
        Some(GraphObjectHandle::new(self))
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        self.source.compile(codegen, inputs)
    }
}

pub struct PureNumberSourceHandle<T: PureNumberSource> {
    instance: Arc<PureNumberSourceWithId<T>>,
}

impl<T: PureNumberSource> PureNumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<PureNumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub fn id(&self) -> NumberSourceId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle {
        GraphObjectHandle::new(self.instance)
    }
}

impl<T: PureNumberSource> Deref for PureNumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: PureNumberSource> Clone for PureNumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

pub struct ScalarInputNumberSource {
    function: ScalarReadFunc,
    input_id: SoundInputId,
}

impl ScalarInputNumberSource {
    pub(super) fn new(input_id: SoundInputId, function: ScalarReadFunc) -> ScalarInputNumberSource {
        ScalarInputNumberSource { function, input_id }
    }
}

impl NumberSource for ScalarInputNumberSource {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        let frame = context.find_input_frame(self.input_id);
        let s = (self.function)(frame.state());
        numeric::fill(dst, s);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen.build_input_scalar_read(self.input_id, self.function)
    }
}

pub struct ArrayInputNumberSource {
    function: ArrayReadFunc,
    input_id: SoundInputId,
}

impl ArrayInputNumberSource {
    pub(super) fn new(input_id: SoundInputId, function: ArrayReadFunc) -> ArrayInputNumberSource {
        ArrayInputNumberSource { function, input_id }
    }
}

impl NumberSource for ArrayInputNumberSource {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        let frame = context.find_input_frame(self.input_id);
        let s = (self.function)(frame.state());
        numeric::copy(s, dst);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen.build_input_array_read(self.input_id, self.function)
    }
}

pub struct ScalarProcessorNumberSource {
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

impl NumberSource for ScalarProcessorNumberSource {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        let state = context.find_processor_state(self.processor_id);
        let s = (self.function)(&state);
        numeric::fill(dst, s);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen.build_processor_scalar_read(self.processor_id, self.function)
    }
}

pub struct ArrayProcessorNumberSource {
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

impl NumberSource for ArrayProcessorNumberSource {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        let state = context.find_processor_state(self.processor_id);
        let s = (self.function)(&state);
        numeric::copy(s, dst);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen.build_processor_array_read(self.processor_id, self.function)
    }
}

pub struct ProcessorTimeNumberSource {
    processor_id: SoundProcessorId,
}

impl ProcessorTimeNumberSource {
    pub(super) fn new(processor_id: SoundProcessorId) -> ProcessorTimeNumberSource {
        ProcessorTimeNumberSource { processor_id }
    }
}

impl NumberSource for ProcessorTimeNumberSource {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        context.current_time_at_sound_processor(self.processor_id, dst);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        _inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        codegen.build_processor_time(self.processor_id)
    }
}

pub struct InputTimeNumberSource {
    input_id: SoundInputId,
}

impl InputTimeNumberSource {
    pub(super) fn new(input_id: SoundInputId) -> InputTimeNumberSource {
        InputTimeNumberSource { input_id }
    }
}

impl NumberSource for InputTimeNumberSource {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        context.current_time_at_sound_input(self.input_id, dst);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        _inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        todo!()
    }
}

pub struct StateNumberSourceHandle {
    id: NumberSourceId,
}

impl StateNumberSourceHandle {
    pub(super) fn new(id: NumberSourceId) -> StateNumberSourceHandle {
        StateNumberSourceHandle { id }
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }
}
