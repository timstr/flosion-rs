use std::{ops::Deref, sync::Arc};

use super::{
    context::Context,
    graphobject::{GraphObject, ObjectInitialization, ObjectType, WithObjectType},
    nodeallocator::NodeAllocator,
    serialization::Serializer,
    soundchunk::SoundChunk,
    soundprocessortools::SoundProcessorTools,
    statetree::{
        DynamicProcessorNode, ProcessorNodeWrapper, SoundProcessorInput, State, StateAndTiming,
    },
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

pub enum StreamRequest {
    Continue,
    Release { sample_offset: usize },
}

#[derive(PartialEq, Eq)]
pub enum StreamStatus {
    Playing,
    Done,
}

pub trait StaticSoundProcessor: 'static + Sync + Send + WithObjectType {
    type InputType: SoundProcessorInput;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn process_audio(
        &self,
        input: &mut <Self::InputType as SoundProcessorInput>::NodeType,
        dst: &mut SoundChunk,
        context: Context,
    );

    fn serialize(&self, _serializer: Serializer) {}
}

pub trait DynamicSoundProcessor: 'static + Sync + Send + WithObjectType {
    type StateType: State;

    type InputType: SoundProcessorInput;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn get_input(&self) -> &Self::InputType
    where
        Self: Sized;

    fn make_state(&self) -> Self::StateType;

    fn process_audio(
        state: &mut StateAndTiming<Self::StateType>,
        inputs: &mut <Self::InputType as SoundProcessorInput>::NodeType,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus;

    fn serialize(&self, _serializer: Serializer) {}
}

pub struct StaticSoundProcessorWithId<T: StaticSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { processor, id }
    }

    pub fn instance(&self) -> &T {
        &self.processor
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }
}

impl<T: StaticSoundProcessor> WithObjectType for StaticSoundProcessorWithId<T> {
    const TYPE: ObjectType = T::TYPE;
}

pub struct DynamicSoundProcessorWithId<T: DynamicSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { processor, id }
    }

    pub fn instance(&self) -> &T {
        &self.processor
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }
}

impl<T: DynamicSoundProcessor> WithObjectType for DynamicSoundProcessorWithId<T> {
    const TYPE: ObjectType = T::TYPE;
}

pub trait SoundProcessor: 'static + Sync + Send {
    fn serialize(&self, serializer: Serializer);

    fn is_static(&self) -> bool;

    fn as_graph_object(self: Arc<Self>) -> Box<dyn GraphObject>;

    fn make_node(&self, allocator: &NodeAllocator) -> Box<dyn ProcessorNodeWrapper>;
}

impl<T: StaticSoundProcessor> SoundProcessor for StaticSoundProcessorWithId<T> {
    fn serialize(&self, serializer: Serializer) {
        self.processor.serialize(serializer);
    }

    fn is_static(&self) -> bool {
        true
    }

    fn as_graph_object(self: Arc<Self>) -> Box<dyn GraphObject> {
        let h = StaticSoundProcessorHandle::new(Arc::clone(&self));
        Box::new(h)
    }

    fn make_node(&self, allocator: &NodeAllocator) -> Box<dyn ProcessorNodeWrapper> {
        // TODO: grab cached output from sound graph topology's static processor cache
        todo!()
    }
}

impl<T: DynamicSoundProcessor> SoundProcessor for DynamicSoundProcessorWithId<T> {
    fn serialize(&self, serializer: Serializer) {
        self.processor.serialize(serializer);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn as_graph_object(self: Arc<Self>) -> Box<dyn GraphObject> {
        let h = DynamicSoundProcessorHandle::new(Arc::clone(&self));
        Box::new(h)
    }

    fn make_node(&self, allocator: &NodeAllocator) -> Box<dyn ProcessorNodeWrapper> {
        // TODO: make allocator aware of synchronous groups and track shared nodes
        let input_node = self.processor.get_input().make_node(allocator);
        let processor_node = DynamicProcessorNode::<T>::new(
            allocator.processor_id(),
            self.processor.make_state(),
            input_node,
        );
        Box::new(processor_node)
    }
}

pub struct StaticSoundProcessorHandle<T: StaticSoundProcessor> {
    processor: Arc<StaticSoundProcessorWithId<T>>,
}

impl<T: StaticSoundProcessor> Clone for StaticSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            processor: Arc::clone(&self.processor),
        }
    }
}

impl<T: StaticSoundProcessor> StaticSoundProcessorHandle<T> {
    pub(crate) fn new(processor: Arc<StaticSoundProcessorWithId<T>>) -> Self {
        Self { processor }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    pub fn instance(&self) -> &T {
        &self.processor.processor
    }

    pub fn as_graph_object(&self) -> Box<dyn GraphObject> {
        Box::new(self.clone())
    }
}

impl<T: StaticSoundProcessor> Deref for StaticSoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.processor.processor
    }
}

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    processor: Arc<DynamicSoundProcessorWithId<T>>,
}

impl<T: DynamicSoundProcessor> Clone for DynamicSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            processor: Arc::clone(&self.processor),
        }
    }
}

impl<T: DynamicSoundProcessor> Deref for DynamicSoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.processor.processor
    }
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub(crate) fn new(processor: Arc<DynamicSoundProcessorWithId<T>>) -> Self {
        Self { processor }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    pub fn instance(&self) -> &T {
        &self.processor.processor
    }

    pub fn as_graph_object(&self) -> Box<dyn GraphObject> {
        Box::new(self.clone())
    }
}
