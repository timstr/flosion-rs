use std::{
    any::{type_name, Any},
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

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
    expression::ProcessorExpression,
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

pub trait WhateverSoundProcessor: Sized + WithObjectType {
    type StateType: State;

    type SoundInputType: SoundProcessorInput;

    // TODO: remove this, compile expressions automatically using
    // generic visitor method below, store compiled expressions in
    // a simple key/value container or array
    type Expressions<'ctx>: CompiledExpressionCollection<'ctx>;

    fn new(tools: SoundProcessorTools, args: &ParsedArguments) -> Result<Self, ()>;

    fn is_static(&self) -> bool;

    fn get_sound_input(&self) -> &Self::SoundInputType;

    fn make_state(&self) -> Self::StateType;

    // TODO: consider generalizing this to take some kind of trait
    // object which can visit expressions, sound inputs, processor
    // arguments, and sound input arguments. This would replace
    // the need for get_sound_input and would be a big step towards
    // relaxing the many weird restrictions on this interface.
    fn visit_expressions<'a>(&self, f: Box<dyn 'a + FnMut(&ProcessorExpression)>);
    fn visit_expressions_mut<'a>(&mut self, f: Box<dyn 'a + FnMut(&mut ProcessorExpression)>);

    // TODO:
    fn compile_expressions<'a, 'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx>;

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<Self::StateType>,
        sound_inputs: &mut <Self::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
        expressions: &mut Self::Expressions<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus;
}

pub struct WhateverSoundProcessorWithId<T: WhateverSoundProcessor> {
    processor: RefCell<T>,
    id: SoundProcessorId,
    time_argument: SoundExpressionArgumentId,
}

impl<T: WhateverSoundProcessor> WhateverSoundProcessorWithId<T> {
    pub(crate) fn new(
        processor: T,
        id: SoundProcessorId,
        time_argument: SoundExpressionArgumentId,
    ) -> Self {
        Self {
            processor: RefCell::new(processor),
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

impl<T: WhateverSoundProcessor> WithObjectType for WhateverSoundProcessorWithId<T> {
    const TYPE: ObjectType = T::TYPE;
}

pub struct WhateverSoundProcessorHandle<T: WhateverSoundProcessor> {
    instance: Rc<WhateverSoundProcessorWithId<T>>,
}

// NOTE: Deriving Clone explicitly because #[derive(Clone)] stupidly
// requires T: Clone even if it isn't stored as a direct field
impl<T: WhateverSoundProcessor> Clone for WhateverSoundProcessorHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Rc::clone(&self.instance),
        }
    }
}

impl<T: 'static + WhateverSoundProcessor> WhateverSoundProcessorHandle<T> {
    pub(super) fn new(instance: Rc<WhateverSoundProcessorWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: AnySoundObjectHandle) -> Option<Self> {
        let rc_any = handle.into_instance_rc().into_rc_any();
        match rc_any.downcast::<WhateverSoundProcessorWithId<T>>() {
            Ok(obj) => Some(WhateverSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> AnySoundObjectHandle {
        AnySoundObjectHandle::new(self.instance)
    }

    pub fn get<'a>(&'a self) -> impl 'a + Deref<Target = T> {
        self.instance.processor.borrow()
    }

    pub fn get_mut<'a>(&'a self) -> impl 'a + DerefMut<Target = T> {
        self.instance.processor.borrow_mut()
    }
}

pub(crate) trait SoundProcessor {
    fn id(&self) -> SoundProcessorId;

    fn is_static(&self) -> bool;

    fn as_graph_object(self: Rc<Self>) -> AnySoundObjectHandle;

    fn visit_expressions<'a>(&self, f: Box<dyn 'a + FnMut(&ProcessorExpression)>);

    fn visit_expressions_mut<'a>(&self, f: Box<dyn 'a + FnMut(&mut ProcessorExpression)>);

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>;
}

// TODO: remove this and merge with what is currently known as WhateverSoundProcessor.
impl<T: 'static + WhateverSoundProcessor> SoundProcessor for WhateverSoundProcessorWithId<T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn is_static(&self) -> bool {
        T::is_static(&self.processor.borrow())
    }

    fn as_graph_object(self: Rc<Self>) -> AnySoundObjectHandle {
        AnySoundObjectHandle::new(self)
    }

    fn visit_expressions<'a>(&self, f: Box<dyn 'a + FnMut(&ProcessorExpression)>) {
        T::visit_expressions(&self.processor.borrow(), f);
    }

    fn visit_expressions_mut<'a>(&self, f: Box<dyn 'a + FnMut(&mut ProcessorExpression)>) {
        T::visit_expressions_mut(&mut self.processor.borrow_mut(), f);
    }

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        if self.is_static() {
            Box::new(CompiledStaticProcessor::new(
                self.id,
                &*self.processor.borrow(),
                compiler,
            ))
        } else {
            Box::new(CompiledDynamicProcessor::new(
                self.id,
                &*self.processor.borrow(),
                compiler,
            ))
        }
    }
}

pub struct ProcessorTiming {
    elapsed_chunks: usize,
}

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

impl<T: 'static + WhateverSoundProcessor> SoundGraphObject for WhateverSoundProcessorWithId<T> {
    fn create(graph: &mut SoundGraph, args: &ParsedArguments) -> Result<AnySoundObjectHandle, ()> {
        graph
            .add_sound_processor::<T>(args)
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
}

pub trait ProcessorHandle {
    fn id(&self) -> SoundProcessorId;

    fn time_argument(&self) -> SoundExpressionArgumentId;
}

impl<T: 'static + WhateverSoundProcessor> ProcessorHandle for WhateverSoundProcessorHandle<T> {
    fn id(&self) -> SoundProcessorId {
        WhateverSoundProcessorHandle::id(self)
    }

    fn time_argument(&self) -> SoundExpressionArgumentId {
        self.instance.time_argument()
    }
}

impl<T: 'static + WhateverSoundProcessor> SoundObjectHandle for WhateverSoundProcessorHandle<T> {
    type ObjectType = WhateverSoundProcessorWithId<T>;

    fn from_graph_object(object: AnySoundObjectHandle) -> Option<Self> {
        WhateverSoundProcessorHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}
