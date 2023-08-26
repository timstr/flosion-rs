use std::{
    any::{type_name, Any},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use serialization::Serializer;

use crate::core::{
    engine::{
        nodegen::NodeGen,
        stategraphnode::{DynamicProcessorNode, StateGraphNode, StaticProcessorNode},
    },
    graph::graphobject::{
        GraphObject, GraphObjectHandle, ObjectHandle, ObjectInitialization, ObjectType,
        WithObjectType,
    },
    soundchunk::SoundChunk,
    uniqueid::UniqueId,
};

use super::{
    context::Context, soundgraph::SoundGraph, soundgraphid::SoundObjectId,
    soundinputnode::SoundProcessorInput, soundnumberinputnode::SoundNumberInputNodeCollection,
    soundprocessortools::SoundProcessorTools, state::State,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundProcessorId(usize);

impl SoundProcessorId {
    pub(crate) fn new(id: usize) -> SoundProcessorId {
        SoundProcessorId(id)
    }
}

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

    type NumberInputType<'ctx>: SoundNumberInputNodeCollection<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx>;

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

    type NumberInputType<'ctx>: SoundNumberInputNodeCollection<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn make_state(&self) -> Self::StateType;

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
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

pub struct StaticSoundProcessorWithId<T: StaticSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { processor, id }
    }

    pub fn id(&self) -> SoundProcessorId {
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

pub struct DynamicSoundProcessorWithId<T: DynamicSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { processor, id }
    }

    pub fn id(&self) -> SoundProcessorId {
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

    pub(super) fn from_graph_object(handle: GraphObjectHandle<SoundGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<StaticSoundProcessorWithId<T>>() {
            Ok(obj) => Some(StaticSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle<SoundGraph> {
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

    pub(super) fn from_graph_object(handle: GraphObjectHandle<SoundGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<DynamicSoundProcessorWithId<T>>() {
            Ok(obj) => Some(DynamicSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle<SoundGraph> {
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

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<SoundGraph>;

    fn make_node<'a, 'ctx>(
        self: Arc<Self>,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Box<dyn 'ctx + StateGraphNode<'ctx>>;
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

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<SoundGraph> {
        GraphObjectHandle::new(self)
    }

    fn make_node<'a, 'ctx>(
        self: Arc<Self>,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Box<dyn 'ctx + StateGraphNode<'ctx>> {
        let processor_node = StaticProcessorNode::new(Arc::clone(&self), nodegen);
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

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<SoundGraph> {
        GraphObjectHandle::new(self)
    }

    fn make_node<'a, 'ctx>(
        self: Arc<Self>,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Box<dyn 'ctx + StateGraphNode<'ctx>> {
        let processor_node = DynamicProcessorNode::new(&*self, nodegen);
        Box::new(processor_node)
    }
}

pub struct ProcessorTiming {
    elapsed_chunks: usize,
}

// TODO: somehow make this available for static processor also?
impl ProcessorTiming {
    fn new() -> ProcessorTiming {
        ProcessorTiming { elapsed_chunks: 0 }
    }

    fn reset(&mut self) {
        self.elapsed_chunks = 0;
    }

    pub(crate) fn advance_one_chunk(&mut self) {
        self.elapsed_chunks += 1;
    }

    pub(super) fn elapsed_chunks(&self) -> usize {
        self.elapsed_chunks
    }

    fn just_started(&self) -> bool {
        self.elapsed_chunks == 0
    }
}

pub struct StateAndTiming<T: State> {
    state: T,
    pub(crate) timing: ProcessorTiming,
}

pub trait ProcessorState: 'static + Sync + Send {
    fn state(&self) -> &dyn Any;

    fn is_static(&self) -> bool;

    fn timing(&self) -> Option<&ProcessorTiming>;

    fn timing_mut(&mut self) -> Option<&mut ProcessorTiming>;

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

    fn timing_mut(&mut self) -> Option<&mut ProcessorTiming> {
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

    fn timing_mut(&mut self) -> Option<&mut ProcessorTiming> {
        Some((self as &mut StateAndTiming<T>).timing_mut())
    }

    fn reset(&mut self) {
        self.state.reset();
        self.timing.reset();
    }
}

impl<T: State> StateAndTiming<T> {
    pub(crate) fn new(state: T) -> StateAndTiming<T> {
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

    pub(super) fn timing_mut(&mut self) -> &mut ProcessorTiming {
        &mut self.timing
    }

    pub fn just_started(&self) -> bool {
        self.timing.just_started()
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

impl<T: StaticSoundProcessor> ObjectHandle<SoundGraph> for StaticSoundProcessorHandle<T> {
    type ObjectType = StaticSoundProcessorWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<SoundGraph>) -> Option<Self> {
        StaticSoundProcessorHandle::from_graph_object(object)
    }
}

impl<T: DynamicSoundProcessor> ObjectHandle<SoundGraph> for DynamicSoundProcessorHandle<T> {
    type ObjectType = DynamicSoundProcessorWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<SoundGraph>) -> Option<Self> {
        DynamicSoundProcessorHandle::from_graph_object(object)
    }
}

impl<T: StaticSoundProcessor> GraphObject<SoundGraph> for StaticSoundProcessorWithId<T> {
    fn create(
        graph: &mut SoundGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<SoundGraph>, ()> {
        graph
            .add_static_sound_processor::<T>(init)
            .map(|h| h.into_graph_object())
    }

    fn get_id(&self) -> SoundObjectId {
        self.id().into()
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        (&*self as &T).serialize(serializer);
    }
}

impl<T: DynamicSoundProcessor> GraphObject<SoundGraph> for DynamicSoundProcessorWithId<T> {
    fn create(
        graph: &mut SoundGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<SoundGraph>, ()> {
        graph
            .add_dynamic_sound_processor::<T>(init)
            .map(|h| h.into_graph_object())
    }

    fn get_id(&self) -> SoundObjectId {
        self.id().into()
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        let s: &T = &*self;
        s.serialize(serializer);
    }
}
