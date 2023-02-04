use std::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use super::{
    context::Context,
    graphobject::{GraphObjectHandle, ObjectInitialization, ObjectType, WithObjectType},
    numberinputnode::NumberInputNodeCollection,
    serialization::Serializer,
    soundchunk::SoundChunk,
    soundinputnode::SoundProcessorInput,
    soundprocessortools::SoundProcessorTools,
    state::State,
    stategraphnode::{DynamicProcessorNode, StateGraphNode, StaticProcessorNode},
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundProcessorId(pub usize);

impl Default for SoundProcessorId {
    fn default() -> SoundProcessorId {
        SoundProcessorId(1)
    }
}

impl UniqueId for SoundProcessorId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundProcessorId {
        SoundProcessorId(self.0 + 1)
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum StreamStatus {
    Playing,
    Done,
}

pub trait StaticSoundProcessor: 'static + Sized + Sync + Send + WithObjectType {
    type SoundInputType: SoundProcessorInput;

    type NumberInputType<'ctx>: NumberInputNodeCollection<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn make_number_inputs<'ctx>(&self) -> Self::NumberInputType<'ctx>;

    fn process_audio<'ctx>(
        &self,
        sound_inputs: &mut <Self::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
        number_inputs: &Self::NumberInputType<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    );

    fn serialize(&self, _serializer: Serializer) {}
}

pub trait DynamicSoundProcessor: 'static + Sized + Sync + Send + WithObjectType {
    type StateType: State;

    type SoundInputType: SoundProcessorInput;

    type NumberInputType<'ctx>: NumberInputNodeCollection<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn make_state(&self) -> Self::StateType;

    fn make_number_inputs<'ctx>(
        &self,
        context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx>;

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<Self::StateType>,
        sound_inputs: &mut <Self::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
        number_inputs: &Self::NumberInputType<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus;

    fn serialize(&self, _serializer: Serializer) {}
}

pub(crate) struct StaticSoundProcessorWithId<T: StaticSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { processor, id }
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.id
    }
}

impl<T: StaticSoundProcessor> Deref for StaticSoundProcessorWithId<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.processor
    }
}

impl<T: StaticSoundProcessor> WithObjectType for StaticSoundProcessorWithId<T> {
    const TYPE: ObjectType = T::TYPE;
}

pub(crate) struct DynamicSoundProcessorWithId<T: DynamicSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { processor, id }
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.id
    }
}

impl<T: DynamicSoundProcessor> Deref for DynamicSoundProcessorWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.processor
    }
}

impl<T: DynamicSoundProcessor> WithObjectType for DynamicSoundProcessorWithId<T> {
    const TYPE: ObjectType = T::TYPE;
}

pub struct StaticSoundProcessorHandle<T: StaticSoundProcessor> {
    instance: Arc<StaticSoundProcessorWithId<T>>,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorHandle<T> {
    pub(super) fn new(instance: Arc<StaticSoundProcessorWithId<T>>) -> Self {
        Self { instance }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle {
        GraphObjectHandle::new(self.instance)
    }
}

impl<T: StaticSoundProcessor> Deref for StaticSoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: StaticSoundProcessor> Clone for StaticSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    instance: Arc<DynamicSoundProcessorWithId<T>>,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub(super) fn new(instance: Arc<DynamicSoundProcessorWithId<T>>) -> Self {
        Self { instance }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle {
        GraphObjectHandle::new(self.instance)
    }
}

impl<T: DynamicSoundProcessor> Deref for DynamicSoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: DynamicSoundProcessor> Clone for DynamicSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

pub(crate) trait SoundProcessor: 'static + Sync + Send {
    fn id(&self) -> SoundProcessorId;

    fn serialize(&self, serializer: Serializer);

    fn is_static(&self) -> bool;

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle;

    fn make_node<'ctx>(
        self: Arc<Self>,
        context: &'ctx inkwell::context::Context,
    ) -> Box<dyn StateGraphNode + 'ctx>;
}

impl<T: StaticSoundProcessor> SoundProcessor for StaticSoundProcessorWithId<T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn serialize(&self, serializer: Serializer) {
        self.processor.serialize(serializer);
    }

    fn is_static(&self) -> bool {
        true
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle {
        GraphObjectHandle::new(self)
    }

    fn make_node<'ctx>(
        self: Arc<Self>,
        _context: &'ctx inkwell::context::Context,
    ) -> Box<dyn StateGraphNode + 'ctx> {
        let processor_node = StaticProcessorNode::<T>::new(Arc::clone(&self));
        Box::new(processor_node)
    }
}

impl<T: DynamicSoundProcessor> SoundProcessor for DynamicSoundProcessorWithId<T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn serialize(&self, serializer: Serializer) {
        self.processor.serialize(serializer);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle {
        GraphObjectHandle::new(self)
    }

    fn make_node<'ctx>(
        self: Arc<Self>,
        context: &'ctx inkwell::context::Context,
    ) -> Box<dyn StateGraphNode + 'ctx> {
        let processor_node = DynamicProcessorNode::<T>::new(&*self, context);
        Box::new(processor_node)
    }
}

pub struct ProcessorTiming {
    elapsed_chunks: usize,
}

impl ProcessorTiming {
    fn new() -> ProcessorTiming {
        ProcessorTiming { elapsed_chunks: 0 }
    }

    fn reset(&mut self) {
        self.elapsed_chunks = 0;
    }

    pub(super) fn advance_one_chunk(&mut self) {
        self.elapsed_chunks += 1;
    }

    pub fn elapsed_chunks(&self) -> usize {
        self.elapsed_chunks
    }
}

pub struct StateAndTiming<T: State> {
    state: T,
    pub(super) timing: ProcessorTiming,
}

pub trait ProcessorState: 'static + Sync + Send {
    fn state(&self) -> &dyn Any;

    fn is_static(&self) -> bool;

    fn timing(&self) -> Option<&ProcessorTiming>;

    fn reset(&mut self);
}

impl<T: StaticSoundProcessor> ProcessorState for T {
    fn state(&self) -> &dyn Any {
        self
    }

    fn is_static(&self) -> bool {
        true
    }

    fn timing(&self) -> Option<&ProcessorTiming> {
        None
    }

    fn reset(&mut self) {
        // A static processor can't be reset
    }
}

impl<T: State> ProcessorState for StateAndTiming<T> {
    fn state(&self) -> &dyn Any {
        (self as &StateAndTiming<T>).state()
    }

    fn is_static(&self) -> bool {
        false
    }

    fn timing(&self) -> Option<&ProcessorTiming> {
        Some((self as &StateAndTiming<T>).timing())
    }

    fn reset(&mut self) {
        self.state.reset();
        self.timing.reset();
    }
}

impl<T: State> StateAndTiming<T> {
    pub(super) fn new(state: T) -> StateAndTiming<T> {
        StateAndTiming {
            state,
            timing: ProcessorTiming::new(),
        }
    }

    pub fn state(&self) -> &T {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    pub(super) fn timing(&self) -> &ProcessorTiming {
        &self.timing
    }
}

impl<T: State> Deref for StateAndTiming<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T: State> DerefMut for StateAndTiming<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
