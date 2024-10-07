use std::{
    any::{type_name, Any},
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    time::{Duration, Instant},
};

use crate::{
    core::{
        engine::{
            soundgraphcompiler::SoundGraphCompiler,
            stategraphnode::{AnyCompiledProcessorData, CompiledProcessorData},
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
    expressionargument::{ProcessorArgument, SoundInputArgument},
    soundgraph::SoundGraph,
    soundgraphid::SoundObjectId,
    soundinput::{ProcessorInput, ProcessorInputId},
    soundobject::{AnySoundObjectHandle, SoundGraphObject, SoundObjectHandle},
    soundprocessortools::SoundProcessorTools,
};

pub struct SoundProcessorTag;

pub type SoundProcessorId = UniqueId<SoundProcessorTag>;

#[derive(PartialEq, Eq, Copy, Clone)]
pub enum StreamStatus {
    Playing,
    Done,
}

pub trait CompiledProcessorComponent<'ctx>: Send {
    fn start_over(&mut self);
}

impl<'ctx> CompiledProcessorComponent<'ctx> for () {
    fn start_over(&mut self) {}
}

pub trait ProcessorComponent {
    type CompiledType<'ctx>: CompiledProcessorComponent<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor);
    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut);

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx>;

    // TODO: for variable-length things like sound input lists,
    // lists of expressions, and variable sound input branches,
    // add a mechanism for partial recompilation and updating.
    // For now, just recompile and replace everything when something
    // changes.
}

pub trait ProcessorComponentVisitor {
    fn input(&mut self, _input: &ProcessorInput) {}
    fn expression(&mut self, _expression: &ProcessorExpression) {}
    fn processor_argument(&mut self, _argument: &ProcessorArgument) {}
    fn input_argument(&mut self, _argument: &SoundInputArgument, _input_id: ProcessorInputId) {}
}

pub trait ProcessorComponentVisitorMut {
    fn input(&mut self, _input: &mut ProcessorInput) {}
    fn expression(&mut self, _expression: &mut ProcessorExpression) {}
    fn processor_argument(&mut self, _argument: &mut ProcessorArgument) {}
    fn input_argument(&mut self, _argument: &mut SoundInputArgument, _input_id: ProcessorInputId) {}
}

pub trait WhateverCompiledSoundProcessor<'ctx>: Send {
    fn process_audio(&mut self, dst: &mut SoundChunk, context: Context) -> StreamStatus;

    fn start_over(&mut self);
}

pub trait WhateverSoundProcessor: Sized + WithObjectType {
    type CompiledType<'ctx>: WhateverCompiledSoundProcessor<'ctx>;

    fn new(tools: SoundProcessorTools, args: &ParsedArguments) -> Self;

    fn is_static(&self) -> bool;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor);
    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut);

    fn compile<'ctx>(
        &self,
        id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx>;
}

pub struct WhateverSoundProcessorWithId<T: WhateverSoundProcessor> {
    processor: RefCell<T>,
    id: SoundProcessorId,
}

impl<T: WhateverSoundProcessor> WhateverSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self {
            processor: RefCell::new(processor),
            id,
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
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

    fn visit(&self, visitor: &mut dyn ProcessorComponentVisitor);

    fn visit_mut(&self, visitor: &mut dyn ProcessorComponentVisitorMut);

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>>;
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

    fn visit(&self, visitor: &mut dyn ProcessorComponentVisitor) {
        T::visit(&self.processor.borrow(), visitor);
    }

    fn visit_mut(&self, visitor: &mut dyn ProcessorComponentVisitorMut) {
        T::visit_mut(&mut self.processor.borrow_mut(), visitor);
    }

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>> {
        let start = Instant::now();
        let compiled_processor = self.processor.borrow().compile(self.id, compiler);
        let finish = Instant::now();
        let time_to_compile: Duration = finish - start;
        let time_to_compile_ms = time_to_compile.as_millis();
        if time_to_compile_ms > 10 {
            println!(
                "Compiling static sound processor {} took {} ms",
                self.id.value(),
                time_to_compile_ms
            );
        }
        Box::new(CompiledProcessorData::new(self.id, compiled_processor))
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
}

impl<T: 'static + WhateverSoundProcessor> ProcessorHandle for WhateverSoundProcessorHandle<T> {
    fn id(&self) -> SoundProcessorId {
        WhateverSoundProcessorHandle::id(self)
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
