use std::{
    any::{type_name, Any},
    ops::{Deref, DerefMut},
    rc::Rc,
};

use chive::ChiveIn;

use crate::{
    core::{
        engine::{
            compiledexpression::CompiledExpressionCollection,
            compiledsoundinput::SoundProcessorInput,
            soundgraphcompiler::SoundGraphCompiler,
            stategraphnode::{
                CompiledDynamicProcessor, CompiledSoundProcessor, CompiledStaticProcessor,
            },
        },
        objecttype::{ObjectType, WithObjectType},
        soundchunk::SoundChunk,
        uniqueid::UniqueId,
    },
    ui_core::arguments::ParsedArguments,
};

use super::{
    context::Context,
    expressionargument::SoundExpressionArgumentId,
    soundgraph::SoundGraph,
    soundgraphid::SoundObjectId,
    soundobject::{AnySoundObjectHandle, SoundGraphObject, SoundObjectHandle},
    soundprocessortools::SoundProcessorTools,
    state::State,
};

pub struct SoundProcessorTag;

pub type SoundProcessorId = UniqueId<SoundProcessorTag>;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum StreamStatus {
    Playing,
    Done,
}

// TODO: do StaticSoundProcessor and DynamicSoundProcessor need to be different traits anymore?
// 'Static' should suffice as a runtime property (which it is everywhere else already)
pub trait StaticSoundProcessor: Sized + WithObjectType {
    type StateType: State;

    type SoundInputType: SoundProcessorInput;

    type Expressions<'ctx>: CompiledExpressionCollection<'ctx>;

    fn new(tools: SoundProcessorTools, args: &ParsedArguments) -> Result<Self, ()>;

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
    );

    fn serialize(&self, _chive_in: ChiveIn) {}
}

pub trait DynamicSoundProcessor: Sized + WithObjectType {
    type StateType: State;

    type SoundInputType: SoundProcessorInput;

    type Expressions<'ctx>: CompiledExpressionCollection<'ctx>;

    fn new(tools: SoundProcessorTools, args: &ParsedArguments) -> Result<Self, ()>;

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

    fn serialize(&self, _chive_in: ChiveIn) {}
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
    instance: Rc<StaticSoundProcessorWithId<T>>,
}

// NOTE: Deriving Clone explicitly because #[derive(Clone)] stupidly
// requires T: Clone even if it isn't stored as a direct field
impl<T: StaticSoundProcessor> Clone for StaticSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Rc::clone(&self.instance),
        }
    }
}

impl<T: 'static + StaticSoundProcessor> StaticSoundProcessorHandle<T> {
    pub(super) fn new(instance: Rc<StaticSoundProcessorWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: AnySoundObjectHandle) -> Option<Self> {
        let rc_any = handle.into_instance_rc().into_rc_any();
        match rc_any.downcast::<StaticSoundProcessorWithId<T>>() {
            Ok(obj) => Some(StaticSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> AnySoundObjectHandle {
        AnySoundObjectHandle::new(self.instance)
    }
}

impl<T: StaticSoundProcessor> Deref for StaticSoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    instance: Rc<DynamicSoundProcessorWithId<T>>,
}

// NOTE: Deriving Clone explicitly because #[derive(Clone)] stupidly
// requires T: Clone even if it isn't stored as a direct field
impl<T: DynamicSoundProcessor> Clone for DynamicSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Rc::clone(&self.instance),
        }
    }
}

impl<T: 'static + DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub(super) fn new(instance: Rc<DynamicSoundProcessorWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: AnySoundObjectHandle) -> Option<Self> {
        let rc_any = handle.into_instance_rc().into_rc_any();
        match rc_any.downcast::<DynamicSoundProcessorWithId<T>>() {
            Ok(obj) => Some(DynamicSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> AnySoundObjectHandle {
        AnySoundObjectHandle::new(self.instance)
    }
}

impl<T: DynamicSoundProcessor> Deref for DynamicSoundProcessorHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

pub(crate) trait SoundProcessor {
    fn id(&self) -> SoundProcessorId;

    fn serialize(&self, chive_in: ChiveIn);

    fn is_static(&self) -> bool;

    fn as_graph_object(self: Rc<Self>) -> AnySoundObjectHandle;

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>;
}

impl<T: 'static + StaticSoundProcessor> SoundProcessor for StaticSoundProcessorWithId<T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn serialize(&self, chive_in: ChiveIn) {
        self.processor.serialize(chive_in);
    }

    fn is_static(&self) -> bool {
        true
    }

    fn as_graph_object(self: Rc<Self>) -> AnySoundObjectHandle {
        AnySoundObjectHandle::new(self)
    }

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        let processor_node = CompiledStaticProcessor::new(self, compiler);
        Box::new(processor_node)
    }
}

impl<T: 'static + DynamicSoundProcessor> SoundProcessor for DynamicSoundProcessorWithId<T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn serialize(&self, chive_in: ChiveIn) {
        self.processor.serialize(chive_in);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn as_graph_object(self: Rc<Self>) -> AnySoundObjectHandle {
        AnySoundObjectHandle::new(self)
    }

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        let processor_node = CompiledDynamicProcessor::new(self, compiler);
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

pub trait ProcessorState: Send {
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

impl<T: 'static + State> ProcessorState for StateAndTiming<T> {
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

impl<T: 'static + StaticSoundProcessor> SoundGraphObject for StaticSoundProcessorWithId<T> {
    fn create(graph: &mut SoundGraph, args: &ParsedArguments) -> Result<AnySoundObjectHandle, ()> {
        graph
            .add_static_sound_processor::<T>(args)
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

    fn into_rc_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, chive_in: ChiveIn) {
        (&*self as &T).serialize(chive_in);
    }
}

impl<T: 'static + DynamicSoundProcessor> SoundGraphObject for DynamicSoundProcessorWithId<T> {
    fn create(graph: &mut SoundGraph, args: &ParsedArguments) -> Result<AnySoundObjectHandle, ()> {
        graph
            .add_dynamic_sound_processor::<T>(args)
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

    fn into_rc_any(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, chive_in: ChiveIn) {
        let s: &T = &*self;
        s.serialize(chive_in);
    }
}

pub trait ProcessorHandle {
    fn id(&self) -> SoundProcessorId;

    fn time_argument(&self) -> SoundExpressionArgumentId;
}

impl<T: 'static + StaticSoundProcessor> ProcessorHandle for StaticSoundProcessorHandle<T> {
    fn id(&self) -> SoundProcessorId {
        StaticSoundProcessorHandle::id(self)
    }

    fn time_argument(&self) -> SoundExpressionArgumentId {
        self.instance.time_argument()
    }
}

impl<T: 'static + DynamicSoundProcessor> ProcessorHandle for DynamicSoundProcessorHandle<T> {
    fn id(&self) -> SoundProcessorId {
        DynamicSoundProcessorHandle::id(self)
    }

    fn time_argument(&self) -> SoundExpressionArgumentId {
        self.instance.time_argument()
    }
}

impl<T: 'static + StaticSoundProcessor> SoundObjectHandle for StaticSoundProcessorHandle<T> {
    type ObjectType = StaticSoundProcessorWithId<T>;

    fn from_graph_object(object: AnySoundObjectHandle) -> Option<Self> {
        StaticSoundProcessorHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}

impl<T: 'static + DynamicSoundProcessor> SoundObjectHandle for DynamicSoundProcessorHandle<T> {
    type ObjectType = DynamicSoundProcessorWithId<T>;

    fn from_graph_object(object: AnySoundObjectHandle) -> Option<Self> {
        DynamicSoundProcessorHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}
