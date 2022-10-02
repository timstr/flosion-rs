use std::{ops::Deref, sync::Arc};

use super::{
    context::Context,
    graphobject::{GraphObject, ObjectInitialization, WithObjectType},
    serialization::Serializer,
    soundchunk::SoundChunk,
    soundprocessortools::SoundProcessorTools,
    statetree::{
        NodeAllocator, ProcessorInput, ProcessorNode, ProcessorNodeWrapper, ProcessorState, State,
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
    StaticNoOutput,
}

pub trait SoundProcessor: 'static + Sync + Send + WithObjectType {
    const IS_STATIC: bool;
    type StateType: State;
    type InputType: ProcessorInput;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Self
    where
        Self: Sized;

    fn get_input(&self) -> &Self::InputType
    where
        Self: Sized;

    fn make_state(&self) -> Self::StateType;

    fn process_audio(
        state: &mut ProcessorState<Self::StateType>,
        inputs: &mut <Self::InputType as ProcessorInput>::NodeType,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus;

    fn serialize(&self, _serializer: Serializer) {}
}

pub trait SoundProcessorWrapper: Sync + Send + 'static {
    fn make_node(&self, allocator: &NodeAllocator) -> Box<dyn ProcessorNodeWrapper>;
    fn is_static(&self) -> bool;
    fn as_graph_object(self: Arc<Self>, id: SoundProcessorId) -> Box<dyn GraphObject>;
}

impl<T: SoundProcessor> SoundProcessorWrapper for T {
    fn make_node(&self, allocator: &NodeAllocator) -> Box<dyn ProcessorNodeWrapper> {
        let input_node = self.get_input().make_node(allocator);
        let processor_node =
            ProcessorNode::<T>::new(allocator.processor_id(), self.make_state(), input_node);
        Box::new(processor_node)
    }

    fn is_static(&self) -> bool {
        Self::IS_STATIC
    }

    fn as_graph_object(self: Arc<Self>, id: SoundProcessorId) -> Box<dyn GraphObject> {
        Box::new(SoundProcessorHandle::new(id, Arc::clone(&self)))
    }
}

pub struct SoundProcessorHandle<T: SoundProcessor> {
    id: SoundProcessorId,
    instance: Arc<T>,
}

impl<T: SoundProcessor> SoundProcessorHandle<T> {
    pub fn new(id: SoundProcessorId, instance: Arc<T>) -> SoundProcessorHandle<T> {
        SoundProcessorHandle { id, instance }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn instance(&self) -> &T {
        &*self.instance
    }

    pub fn instance_arc(&self) -> Arc<T> {
        Arc::clone(&self.instance)
    }
}

impl<T: SoundProcessor> Clone for SoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: SoundProcessor> Deref for SoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.instance()
    }
}
