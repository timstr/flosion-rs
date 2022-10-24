use std::{ops::Deref, sync::Arc};

use super::{
    context::Context,
    graphobject::{GraphObject, ObjectInitialization, ObjectType, WithObjectType},
    serialization::Serializer,
    soundchunk::SoundChunk,
    soundprocessortools::SoundProcessorTools,
    statetree::{
        DynamicProcessorNode, NodeAllocator, ProcessorInput, ProcessorNodeWrapper, State,
        StateAndTiming,
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

// TODO:
// list of akward things that making static and dynamic processors the same
// forces me to do for static processors:
// - define a state type that is usually the unit type () and unused
// - jump through weird synchronization hoops just to give the processor node
//   temporary access to the processor's data
// - provide a value for a pointless boolean IS_STATIC which provides
//   information that should just be part of the type system
// - make_state is kinda pointless too
// What I would prefer:
// - separate patterns for static and dynamic processor which embrace the
//   peculiarities of either while (mostly?) automatically providing a shared
//   interface for both
//     - dynamic processor only:
//         - state type
//         - make_state
//         - sound processor node type
//     - static processor only:
//         - process_audio receives self
//     - both
//         - object type
//         - sound input type
//         - serialization
// - perhaps using a general-purpose SoundProcessor trait that has blanket
//   implementations for DynamicSoundProcessor and StaticSoundProcessor
// - ugh but mutual exclusion of traits grumble grumble, I tried this before
// - please rustc I promise I won't ever have a type that is both static and
//   dynamic
// - is it possible to design this in such a way that there are no conflicting
//   blanket implementations?
// - is there an easy way to achieve something like this without blanket trait
//   implementations?
// - maybe move methods of sound processor, dynamicsoundprocessor, and
//   staticsoundprocessor into three unrelated traits?
// - maybe do something with a pair of generic struct DynamicSoundProcessor<T>
//   and StaticSoundProcessor<T> that both implement a SoundProcessorTrait?
//   Seems promising. The generic parameter T for those two structs might
//   themselves need to implement specific traits, BUT those traits can now be
//   tailored specifically to dynamic and static processors. The current
//   SoundProcessorWrapper trait should be able to be implemented for those
//   DynamicSoundProcessor<T> and StaticSoundProcessor<T>, with little change to
//   its interface.

pub trait StaticSoundProcessor: 'static + Sync + Send + WithObjectType {
    type InputType: ProcessorInput;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    // TODO: how to use input nodes here?
    fn process_audio(
        &self,
        input: &mut <Self::InputType as ProcessorInput>::NodeType,
        dst: &mut SoundChunk,
        context: Context,
    );

    fn serialize(&self, _serializer: Serializer) {}
}

pub trait DynamicSoundProcessor: 'static + Sync + Send + WithObjectType {
    type StateType: State;

    type InputType: ProcessorInput;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn get_input(&self) -> &Self::InputType
    where
        Self: Sized;

    fn make_state(&self) -> Self::StateType;

    fn process_audio(
        state: &mut StateAndTiming<Self::StateType>,
        inputs: &mut <Self::InputType as ProcessorInput>::NodeType,
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
