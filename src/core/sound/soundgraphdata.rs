use std::rc::Rc;

use crate::core::sound::{
    expressionargument::ProcessorArgumentLocation,
    soundprocessor::{ProcessorComponentVisitor, ProcessorComponentVisitorMut},
};

use super::{
    expression::{ProcessorExpression, ProcessorExpressionId, ProcessorExpressionLocation},
    expressionargument::{
        ProcessorArgument, ProcessorArgumentId, SoundInputArgument, SoundInputArgumentId,
        SoundInputArgumentLocation,
    },
    soundinput::{ProcessorInput, ProcessorInputId, SoundInputLocation},
    soundprocessor::{SoundProcessor, SoundProcessorId},
};

#[derive(Clone)]
pub(crate) struct SoundProcessorData {
    id: SoundProcessorId,
    processor: Rc<dyn SoundProcessor>,
}

impl SoundProcessorData {
    pub(super) fn new(
        id: SoundProcessorId,
        processor: Rc<dyn SoundProcessor>,
    ) -> SoundProcessorData {
        SoundProcessorData { id, processor }
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub(crate) fn with_input<R, F: FnMut(&ProcessorInput) -> R>(
        &self,
        input_id: ProcessorInputId,
        f: F,
    ) -> Option<R> {
        struct Visitor<R2, F2> {
            input_id: ProcessorInputId,
            result: Option<R2>,
            f: F2,
        }
        impl<R2, F2: FnMut(&ProcessorInput) -> R2> ProcessorComponentVisitor for Visitor<R2, F2> {
            fn input(&mut self, input: &ProcessorInput) {
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
        self.instance().visit(&mut visitor);
        visitor.result
    }

    pub(crate) fn with_input_mut<R, F: FnMut(&mut ProcessorInput) -> R>(
        &mut self,
        input_id: ProcessorInputId,
        f: F,
    ) -> Option<R> {
        struct Visitor<R2, F2> {
            input_id: ProcessorInputId,
            result: Option<R2>,
            f: F2,
        }
        impl<R2, F2: FnMut(&mut ProcessorInput) -> R2> ProcessorComponentVisitorMut for Visitor<R2, F2> {
            fn input(&mut self, input: &mut ProcessorInput) {
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
        // NOTE: interior mutability here means that we didn't actually need self to be mut
        self.instance().visit_mut(&mut visitor);
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
        self.instance().visit_mut(&mut visitor);
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
        self.instance().visit(&mut visitor);
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
        self.instance().visit(&mut visitor);
        visitor.result
    }

    pub(crate) fn foreach_input<F: FnMut(&ProcessorInput, SoundInputLocation)>(&self, f: F) {
        struct Visitor<F2> {
            processor_id: SoundProcessorId,
            f: F2,
        }

        impl<F2: FnMut(&ProcessorInput, SoundInputLocation)> ProcessorComponentVisitor for Visitor<F2> {
            fn input(&mut self, input: &ProcessorInput) {
                (self.f)(
                    input,
                    SoundInputLocation::new(self.processor_id, input.id()),
                )
            }
        }

        self.instance().visit(&mut Visitor {
            processor_id: self.id,
            f,
        });
    }

    pub(crate) fn foreach_input_mut<F: FnMut(&mut ProcessorInput, SoundInputLocation)>(
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

        self.instance().visit(&mut Visitor {
            processor_id: self.id,
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

        self.instance().visit(&mut Visitor {
            processor_id: self.id,
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

        self.instance().visit(&mut Visitor {
            processor_id: self.id,
            f,
        });
    }

    pub(crate) fn input_locations(&self) -> Vec<SoundInputLocation> {
        let mut locations = Vec::new();
        self.foreach_input(|_, l| locations.push(l));
        locations
    }

    pub(crate) fn instance(&self) -> &dyn SoundProcessor {
        &*self.processor
    }

    pub(crate) fn instance_rc(&self) -> Rc<dyn SoundProcessor> {
        Rc::clone(&self.processor)
    }

    pub(crate) fn friendly_name(&self) -> String {
        format!(
            "{}#{}",
            self.instance_rc().as_graph_object().get_type().name(),
            self.id.value()
        )
    }
}
