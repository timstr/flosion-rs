use std::{marker::PhantomData, ops::Deref, sync::Arc};

use super::{
    context::Context,
    graphobject::{GraphObject, ObjectInitialization, WithObjectType},
    numbersourcetools::NumberSourceTools,
    serialization::Serializer,
    soundinput::SoundInputId,
    soundprocessor::{ProcessorState, SoundProcessorId},
    state::{State, StateOwner},
    uniqueid::UniqueId,
};

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
    fn as_graph_object(self: Arc<Self>, _id: NumberSourceId) -> Option<Box<dyn GraphObject>> {
        None
    }
}

pub trait PureNumberSource: 'static + Sync + Send + WithObjectType {
    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn eval(&self, dst: &mut [f32], context: &Context);

    fn serialize(&self, _serializer: Serializer) {}
}

impl<T: PureNumberSource> NumberSource for T {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        T::eval(self, dst, context)
    }

    fn as_graph_object(self: Arc<Self>, id: NumberSourceId) -> Option<Box<dyn GraphObject>> {
        Some(Box::new(PureNumberSourceHandle::new(id, Arc::clone(&self))))
    }
}

pub struct PureNumberSourceHandle<T: PureNumberSource> {
    id: NumberSourceId,
    instance: Arc<T>,
}

impl<T: PureNumberSource> PureNumberSourceHandle<T> {
    pub(super) fn new(id: NumberSourceId, instance: Arc<T>) -> PureNumberSourceHandle<T> {
        PureNumberSourceHandle { id, instance }
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }

    pub fn instance(&self) -> &T {
        &*self.instance
    }

    pub(super) fn instance_arc(&self) -> Arc<T> {
        Arc::clone(&&self.instance)
    }
}

impl<T: PureNumberSource> Clone for PureNumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: PureNumberSource> Deref for PureNumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.instance()
    }
}

pub trait StateFunction<S>: 'static + Sized + Sync + Send {
    fn apply(&self, dst: &mut [f32], state: &S);
}

impl<S, F: 'static + Sized + Sync + Send> StateFunction<S> for F
where
    F: Fn(&mut [f32], &S),
{
    fn apply(&self, dst: &mut [f32], state: &S) {
        (*self)(dst, state);
    }
}

pub struct ProcessorNumberSource<S: ProcessorState, F: StateFunction<S>> {
    function: F,
    processor_id: SoundProcessorId,
    data: PhantomData<S>,
}

impl<S: ProcessorState, F: StateFunction<S>> ProcessorNumberSource<S, F> {
    pub(super) fn new(processor_id: SoundProcessorId, function: F) -> ProcessorNumberSource<S, F> {
        ProcessorNumberSource {
            function,
            processor_id,
            data: PhantomData::default(),
        }
    }
}

impl<S: ProcessorState, F: StateFunction<S>> NumberSource for ProcessorNumberSource<S, F> {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        let state = context.find_processor_state(self.processor_id);
        let state = state.downcast_if::<S>(self.processor_id).unwrap();
        self.function.apply(dst, state);
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
}

pub struct StateNumberSourceHandle {
    id: NumberSourceId,
    owner: NumberSourceOwner,
}

impl StateNumberSourceHandle {
    pub(super) fn new(id: NumberSourceId, owner: NumberSourceOwner) -> StateNumberSourceHandle {
        StateNumberSourceHandle { id, owner }
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }

    pub(super) fn owner(&self) -> NumberSourceOwner {
        self.owner
    }
}

pub struct KeyedInputNumberSource<S: State, F: StateFunction<S>> {
    input_id: SoundInputId,
    function: F,
    dummy_data: PhantomData<S>,
}

impl<S: State, F: StateFunction<S>> KeyedInputNumberSource<S, F> {
    pub(super) fn new(input_id: SoundInputId, function: F) -> KeyedInputNumberSource<S, F> {
        KeyedInputNumberSource {
            input_id,
            function,
            dummy_data: PhantomData::default(),
        }
    }
}

impl<S: State, F: StateFunction<S>> NumberSource for KeyedInputNumberSource<S, F> {
    fn eval(&self, dst: &mut [f32], context: &Context) {
        let frame = context.find_input_frame(self.input_id);
        self.function
            .apply(dst, frame.state().downcast_if::<S>(self.input_id).unwrap());
    }
}
