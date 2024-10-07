use std::rc::Rc;

use crate::core::sound::soundprocessor::{ProcessorComponentVisitor, ProcessorComponentVisitorMut};

use super::{
    expression::{ProcessorExpression, ProcessorExpressionId},
    expressionargument::{
        ProcessorArgument, ProcessorArgumentId, SoundInputArgument, SoundInputArgumentId,
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

    pub(crate) fn with_expression_mut<F: FnMut(&mut ProcessorExpression)>(
        &mut self,
        id: ProcessorExpressionId,
        f: F,
    ) {
        todo!()
    }

    pub(crate) fn with_processor_argument<F: FnMut(&ProcessorArgument)>(
        &self,
        id: ProcessorArgumentId,
        f: F,
    ) {
        todo!()
    }

    pub(crate) fn with_input_argument<F: FnMut(&SoundInputArgument)>(
        &self,
        input_id: ProcessorInputId,
        arg_id: SoundInputArgumentId,
        f: F,
    ) {
        todo!()
    }

    pub(crate) fn foreach_input<F: FnMut(&ProcessorInput, SoundInputLocation)>(&self, f: F) {
        todo!()
    }

    pub(crate) fn foreach_input_mut<F: FnMut(&mut ProcessorInput, SoundInputLocation)>(
        &self,
        f: F,
    ) {
        todo!()
    }

    pub(crate) fn foreach_expression<F: FnMut(&ProcessorExpression)>(&self, f: F) {
        todo!()
    }

    pub(crate) fn foreach_processor_argument<F: FnMut(&ProcessorArgument)>(&self, f: F) {
        todo!()
    }

    // TODO: probably need to pass input id to closure as well?
    pub(crate) fn foreach_input_argument<F: FnMut(&SoundInputArgument)>(&self, f: F) {
        todo!()
    }

    pub(crate) fn input_locations(&self) -> Vec<SoundInputLocation> {
        todo!()
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
