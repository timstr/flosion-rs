use std::{
    any::{type_name, Any},
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
    expression::{ProcessorExpression, ProcessorExpressionId, ProcessorExpressionLocation},
    expressionargument::{
        ProcessorArgument, ProcessorArgumentId, ProcessorArgumentLocation, SoundInputArgument,
        SoundInputArgumentId, SoundInputArgumentLocation,
    },
    soundgraph::SoundGraph,
    soundgraphid::SoundObjectId,
    soundinput::{BasicProcessorInput, ProcessorInputId, SoundInputLocation},
    soundobject::SoundGraphObject,
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
    fn input(&mut self, _input: &BasicProcessorInput) {}
    fn expression(&mut self, _expression: &ProcessorExpression) {}
    fn processor_argument(&mut self, _argument: &ProcessorArgument) {}
    fn input_argument(&mut self, _argument: &SoundInputArgument, _input_id: ProcessorInputId) {}
}

pub trait ProcessorComponentVisitorMut {
    fn input(&mut self, _input: &mut BasicProcessorInput) {}
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

    fn new(args: &ParsedArguments) -> Self;

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
    id: SoundProcessorId,
    processor: T,
}

impl<T: WhateverSoundProcessor> WhateverSoundProcessorWithId<T> {
    pub(crate) fn new(processor: T, id: SoundProcessorId) -> Self {
        Self { id, processor }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }
}

impl<T: WhateverSoundProcessor> Deref for WhateverSoundProcessorWithId<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.processor
    }
}

impl<T: WhateverSoundProcessor> DerefMut for WhateverSoundProcessorWithId<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.processor
    }
}

impl<T: WhateverSoundProcessor> WithObjectType for WhateverSoundProcessorWithId<T> {
    const TYPE: ObjectType = T::TYPE;
}

pub struct WhateverSoundProcessorHandle<T: WhateverSoundProcessor> {
    instance: Rc<WhateverSoundProcessorWithId<T>>,
}

pub(crate) trait SoundProcessor {
    fn id(&self) -> SoundProcessorId;

    fn is_static(&self) -> bool;

    fn as_graph_object(&self) -> &dyn SoundGraphObject;
    fn as_graph_object_mut(&mut self) -> &mut dyn SoundGraphObject;

    fn visit(&self, visitor: &mut dyn ProcessorComponentVisitor);

    fn visit_mut(&mut self, visitor: &mut dyn ProcessorComponentVisitorMut);

    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;

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

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn is_static(&self) -> bool {
        T::is_static(&self.processor)
    }

    fn as_graph_object(&self) -> &dyn SoundGraphObject {
        self
    }

    fn as_graph_object_mut(&mut self) -> &mut dyn SoundGraphObject {
        self
    }

    fn visit(&self, visitor: &mut dyn ProcessorComponentVisitor) {
        T::visit(&self.processor, visitor);
    }

    fn visit_mut(&mut self, visitor: &mut dyn ProcessorComponentVisitorMut) {
        T::visit_mut(&mut self.processor, visitor);
    }

    fn compile<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>> {
        let start = Instant::now();
        let compiled_processor = self.processor.compile(self.id, compiler);
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

impl<'a> dyn SoundProcessor + 'a {
    pub(crate) fn downcast<T: 'static + WhateverSoundProcessor>(
        &self,
    ) -> Option<&WhateverSoundProcessorWithId<T>> {
        self.as_any().downcast_ref()
    }

    pub(crate) fn downcast_mut<T: 'static + WhateverSoundProcessor>(
        &mut self,
    ) -> Option<&mut WhateverSoundProcessorWithId<T>> {
        self.as_mut_any().downcast_mut()
    }

    pub(crate) fn with_input<R, F: FnMut(&BasicProcessorInput) -> R>(
        &self,
        input_id: ProcessorInputId,
        f: F,
    ) -> Option<R> {
        struct Visitor<R2, F2> {
            input_id: ProcessorInputId,
            result: Option<R2>,
            f: F2,
        }
        impl<R2, F2: FnMut(&BasicProcessorInput) -> R2> ProcessorComponentVisitor for Visitor<R2, F2> {
            fn input(&mut self, input: &BasicProcessorInput) {
                if input.id() == self.input_id {
                    debug_assert!(self.result.is_none());
                    self.result = Some((self.f)(input));
                }
            }
        }
        let mut visitor = Visitor {
            input_id,
            result: None,
            f,
        };
        self.visit(&mut visitor);
        visitor.result
    }

    pub(crate) fn with_input_mut<R, F: FnMut(&mut BasicProcessorInput) -> R>(
        &mut self,
        input_id: ProcessorInputId,
        f: F,
    ) -> Option<R> {
        struct Visitor<R2, F2> {
            input_id: ProcessorInputId,
            result: Option<R2>,
            f: F2,
        }
        impl<R2, F2: FnMut(&mut BasicProcessorInput) -> R2> ProcessorComponentVisitorMut
            for Visitor<R2, F2>
        {
            fn input(&mut self, input: &mut BasicProcessorInput) {
                if input.id() == self.input_id {
                    debug_assert!(self.result.is_none());
                    self.result = Some((self.f)(input));
                }
            }
        }
        let mut visitor = Visitor {
            input_id,
            result: None,
            f,
        };

        self.visit_mut(&mut visitor);
        visitor.result
    }

    pub(crate) fn with_expression<F: FnMut(&ProcessorExpression)>(
        &self,
        id: ProcessorExpressionId,
        f: F,
    ) {
        todo!()
    }

    pub(crate) fn with_expression_mut<R, F: FnMut(&mut ProcessorExpression) -> R>(
        &mut self,
        id: ProcessorExpressionId,
        f: F,
    ) -> Option<R> {
        struct Visitor<F2, R2> {
            id: ProcessorExpressionId,
            f: F2,
            result: Option<R2>,
        }

        impl<R2, F2: FnMut(&mut ProcessorExpression) -> R2> ProcessorComponentVisitorMut
            for Visitor<F2, R2>
        {
            fn expression(&mut self, expression: &mut ProcessorExpression) {
                if expression.id() == self.id {}
                debug_assert!(self.result.is_none());
                self.result = Some((self.f)(expression));
            }
        }

        let mut visitor = Visitor {
            id,
            f,
            result: None,
        };
        self.visit_mut(&mut visitor);
        visitor.result
    }

    pub(crate) fn with_processor_argument<R, F: FnMut(&ProcessorArgument) -> R>(
        &self,
        id: ProcessorArgumentId,
        f: F,
    ) -> Option<R> {
        struct Visitor<F2, R2> {
            id: ProcessorArgumentId,
            f: F2,
            result: Option<R2>,
        }

        impl<R2, F2: FnMut(&ProcessorArgument) -> R2> ProcessorComponentVisitor for Visitor<F2, R2> {
            fn processor_argument(&mut self, argument: &ProcessorArgument) {
                if argument.id() == self.id {
                    debug_assert!(self.result.is_none());
                    self.result = Some((self.f)(argument));
                }
            }
        }

        let mut visitor = Visitor {
            id,
            f,
            result: None,
        };
        self.visit(&mut visitor);
        visitor.result
    }

    pub(crate) fn with_input_argument<R, F: FnMut(&SoundInputArgument) -> R>(
        &self,
        input_id: ProcessorInputId,
        argument_id: SoundInputArgumentId,
        f: F,
    ) -> Option<R> {
        struct Visitor<F2, R2> {
            input_id: ProcessorInputId,
            argument_id: SoundInputArgumentId,
            f: F2,
            result: Option<R2>,
        }

        impl<R2, F2: FnMut(&SoundInputArgument) -> R2> ProcessorComponentVisitor for Visitor<F2, R2> {
            fn input_argument(
                &mut self,
                argument: &SoundInputArgument,
                input_id: ProcessorInputId,
            ) {
                if argument.id() == self.argument_id && input_id == self.input_id {
                    debug_assert!(self.result.is_none());
                    self.result = Some((self.f)(argument));
                }
            }
        }

        let mut visitor = Visitor {
            input_id,
            argument_id,
            f,
            result: None,
        };
        self.visit(&mut visitor);
        visitor.result
    }

    pub(crate) fn foreach_input<F: FnMut(&BasicProcessorInput, SoundInputLocation)>(&self, f: F) {
        struct Visitor<F2> {
            processor_id: SoundProcessorId,
            f: F2,
        }

        impl<F2: FnMut(&BasicProcessorInput, SoundInputLocation)> ProcessorComponentVisitor
            for Visitor<F2>
        {
            fn input(&mut self, input: &BasicProcessorInput) {
                (self.f)(
                    input,
                    SoundInputLocation::new(self.processor_id, input.id()),
                )
            }
        }

        self.visit(&mut Visitor {
            processor_id: self.id(),
            f,
        });
    }

    pub(crate) fn foreach_input_mut<F: FnMut(&mut BasicProcessorInput, SoundInputLocation)>(
        &self,
        f: F,
    ) {
        todo!()
    }

    pub(crate) fn foreach_expression<
        F: FnMut(&ProcessorExpression, ProcessorExpressionLocation),
    >(
        &self,
        f: F,
    ) {
        struct Visitor<F2> {
            processor_id: SoundProcessorId,
            f: F2,
        }

        impl<F2: FnMut(&ProcessorExpression, ProcessorExpressionLocation)> ProcessorComponentVisitor
            for Visitor<F2>
        {
            fn expression(&mut self, expression: &ProcessorExpression) {
                (self.f)(
                    expression,
                    ProcessorExpressionLocation::new(self.processor_id, expression.id()),
                )
            }
        }

        self.visit(&mut Visitor {
            processor_id: self.id(),
            f,
        });
    }

    pub(crate) fn foreach_processor_argument<
        F: FnMut(&ProcessorArgument, ProcessorArgumentLocation),
    >(
        &self,
        f: F,
    ) {
        struct Visitor<F2> {
            processor_id: SoundProcessorId,
            f: F2,
        }

        impl<F2: FnMut(&ProcessorArgument, ProcessorArgumentLocation)> ProcessorComponentVisitor
            for Visitor<F2>
        {
            fn processor_argument(&mut self, argument: &ProcessorArgument) {
                (self.f)(
                    argument,
                    ProcessorArgumentLocation::new(self.processor_id, argument.id()),
                )
            }
        }

        self.visit(&mut Visitor {
            processor_id: self.id(),
            f,
        });
    }

    pub(crate) fn foreach_input_argument<
        F: FnMut(&SoundInputArgument, SoundInputArgumentLocation),
    >(
        &self,
        f: F,
    ) {
        struct Visitor<F2> {
            processor_id: SoundProcessorId,
            f: F2,
        }

        impl<F2: FnMut(&SoundInputArgument, SoundInputArgumentLocation)> ProcessorComponentVisitor
            for Visitor<F2>
        {
            fn input_argument(
                &mut self,
                argument: &SoundInputArgument,
                input_id: ProcessorInputId,
            ) {
                (self.f)(
                    argument,
                    SoundInputArgumentLocation::new(self.processor_id, input_id, argument.id()),
                )
            }
        }

        self.visit(&mut Visitor {
            processor_id: self.id(),
            f,
        });
    }

    pub(crate) fn input_locations(&self) -> Vec<SoundInputLocation> {
        let mut locations = Vec::new();
        self.foreach_input(|_, l| locations.push(l));
        locations
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
    fn create<'a>(
        graph: &'a mut SoundGraph,
        args: &ParsedArguments,
    ) -> &'a mut WhateverSoundProcessorWithId<T> {
        graph.add_sound_processor::<T>(args)
    }

    fn id(&self) -> SoundObjectId {
        WhateverSoundProcessorWithId::id(self).into()
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn friendly_name(&self) -> String {
        format!("{}#{}", T::TYPE.name(), self.id.value())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}
