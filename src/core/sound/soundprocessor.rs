use std::{
    any::{type_name, Any},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use serialization::Serializer;

use crate::core::{
    engine::{
        compiledexpression::CompiledExpressionCollection,
        compiledsoundinput::SoundProcessorInput,
        soundgraphcompiler::SoundGraphCompiler,
        stategraphnode::{
            CompiledDynamicProcessor, CompiledSoundProcessor, CompiledStaticProcessor,
        },
    },
    graph::graphobject::{
        GraphObject, GraphObjectHandle, ObjectHandle, ObjectInitialization, ObjectType,
        WithObjectType,
    },
    soundchunk::SoundChunk,
    uniqueid::UniqueId,
};

use super::{
    context::Context, expressionargument::SoundExpressionArgumentId, soundgraph::SoundGraph,
    soundgraphid::SoundObjectId, soundprocessortools::SoundProcessorTools, state::State,
};

pub struct SoundProcessorTag;

pub type SoundProcessorId = UniqueId<SoundProcessorTag>;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum StreamStatus {
    Playing,
    Done,
}

pub trait StaticSoundProcessor: 'static + Sized + Sync + Send + WithObjectType {
    type SoundInputType: SoundProcessorInput;

    type Expressions<'ctx>: CompiledExpressionCollection<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn compile_expressions<'a, 'ctx>(
        &self,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx>;

    fn process_audio<'ctx>(
        processor: &StaticSoundProcessorWithId<Self>,
        timing: &ProcessorTiming,
        sound_inputs: &mut <Self::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
        expressions: &mut Self::Expressions<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    );

    fn serialize(&self, _serializer: Serializer) {}
}

pub trait DynamicSoundProcessor: 'static + Sized + Sync + Send + WithObjectType {
    type StateType: State;

    type SoundInputType: SoundProcessorInput;

    type Expressions<'ctx>: CompiledExpressionCollection<'ctx>;

    fn new(tools: SoundProcessorTools, init: ObjectInitialization) -> Result<Self, ()>;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn make_state(&self) -> Self::StateType;

    fn compile_expressions<'a, 'ctx>(
        &self,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx>;

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<Self::StateType>,
        sound_inputs: &mut <Self::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
        expressions: &mut Self::Expressions<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus;

    fn serialize(&self, _serializer: Serializer) {}
}

pub struct StaticSoundProcessorWithId<T: StaticSoundProcessor> {
    processor: T,
    id: SoundProcessorId,
    time_argument: SoundExpressionArgumentId,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorWithId<T> {
    pub(crate) fn new(
        processor: T,
        id: SoundProcessorId,
        time_argument: SoundExpressionArgumentId,
    ) -> Self {
        Self {
            processor,
            id,
            time_argument,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn time_argument(&self) -> SoundExpressionArgumentId {
        self.time_argument
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
    time_argument: SoundExpressionArgumentId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorWithId<T> {
    pub(crate) fn new(
        processor: T,
        id: SoundProcessorId,
        time_argument: SoundExpressionArgumentId,
    ) -> Self {
        Self {
            processor,
            id,
            time_argument,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn time_argument(&self) -> SoundExpressionArgumentId {
        self.time_argument
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

    fn compile<'a, 'ctx>(
        self: Arc<Self>,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>;
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

    fn compile<'a, 'ctx>(
        self: Arc<Self>,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        let processor_node = CompiledStaticProcessor::new(Arc::clone(&self), compiler);
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

    fn compile<'a, 'ctx>(
        self: Arc<Self>,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        let processor_node = CompiledDynamicProcessor::new(&*self, compiler);
        Box::new(processor_node)
    }
}

pub struct ProcessorTiming {
    elapsed_chunks: usize,
}

// TODO: somehow make this available for static processor also?
impl ProcessorTiming {
    pub(crate) fn new() -> ProcessorTiming {
        ProcessorTiming { elapsed_chunks: 0 }
    }

    pub(crate) fn start_over(&mut self) {
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

    fn timing(&self) -> &ProcessorTiming;

    fn start_over(&mut self);
}

impl ProcessorState for ProcessorTiming {
    fn state(&self) -> &dyn Any {
        self
    }

    fn is_static(&self) -> bool {
        true
    }

    fn timing(&self) -> &ProcessorTiming {
        self
    }

    fn start_over(&mut self) {
        (self as &mut ProcessorTiming).start_over();
    }
}

impl<T: State> ProcessorState for StateAndTiming<T> {
    fn state(&self) -> &dyn Any {
        (self as &StateAndTiming<T>).state()
    }

    fn is_static(&self) -> bool {
        false
    }

    fn timing(&self) -> &ProcessorTiming {
        (self as &StateAndTiming<T>).timing()
    }

    fn start_over(&mut self) {
        self.state.start_over();
        self.timing.start_over();
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

    fn object_type() -> ObjectType {
        T::TYPE
    }
}

impl<T: DynamicSoundProcessor> ObjectHandle<SoundGraph> for DynamicSoundProcessorHandle<T> {
    type ObjectType = DynamicSoundProcessorWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<SoundGraph>) -> Option<Self> {
        DynamicSoundProcessorHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
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
            .map_err(|_| ()) // TODO: report error
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
            .map_err(|_| ()) // TODO: report error
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

pub trait ProcessorHandle {
    fn id(&self) -> SoundProcessorId;

    fn time_argument(&self) -> SoundExpressionArgumentId;
}

impl<T: StaticSoundProcessor> ProcessorHandle for StaticSoundProcessorHandle<T> {
    fn id(&self) -> SoundProcessorId {
        StaticSoundProcessorHandle::id(self)
    }

    fn time_argument(&self) -> SoundExpressionArgumentId {
        self.instance.time_argument()
    }
}

impl<T: DynamicSoundProcessor> ProcessorHandle for DynamicSoundProcessorHandle<T> {
    fn id(&self) -> SoundProcessorId {
        DynamicSoundProcessorHandle::id(self)
    }

    fn time_argument(&self) -> SoundExpressionArgumentId {
        self.instance.time_argument()
    }
}
