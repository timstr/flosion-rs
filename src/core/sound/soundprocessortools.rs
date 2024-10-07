use std::rc::Rc;

use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    uniqueid::IdGenerator,
};

use super::{
    expression::{ProcessorExpression, ProcessorExpressionId, SoundExpressionScope},
    expressionargument::{
        AnySoundInputArgument, ArrayProcessorExpressionArgument, ProcessorArgument,
        ProcessorArgumentId, ProcessorLocalArrayExpressionArgument,
        ScalarProcessorExpressionArgument, SoundInputArgument, SoundInputArgumentId,
    },
    soundinput::{InputOptions, ProcessorInput, ProcessorInputId, SoundInputBranchId},
    soundprocessor::SoundProcessorId,
};

/// An interface for making changes to the sound graph from the view of
/// a single sound processor. Changes to a single
/// processor, such as adding and removing sound inputs and modifying
/// expressions and expression arguments, can be done through SoundProcessorTools.
/// This is largely for convenience, since doing the same changes through
/// the SoundGraph interface directly would mean to pass around the
/// processor's id and thus be juggling even more ids at once.
pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,

    // TODO: borrow this from the graph
    input_idgen: &'a mut IdGenerator<ProcessorInputId>,
    expression_idgen: &'a mut IdGenerator<ProcessorExpressionId>,
    proc_arg_idgen: &'a mut IdGenerator<ProcessorArgumentId>,
    input_arg_idgen: &'a mut IdGenerator<SoundInputArgumentId>,
}

impl<'a> SoundProcessorTools<'a> {
    pub(super) fn new(
        id: SoundProcessorId,

        input_idgen: &'a mut IdGenerator<ProcessorInputId>,
        expression_idgen: &'a mut IdGenerator<ProcessorExpressionId>,
        proc_arg_idgen: &'a mut IdGenerator<ProcessorArgumentId>,
        input_arg_idgen: &'a mut IdGenerator<SoundInputArgumentId>,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
            input_idgen,
            expression_idgen,
            proc_arg_idgen,
            input_arg_idgen,
        }
    }

    /// Add a sound input to the sound processor with the given
    /// options and list of branches. Usually you do not want to
    /// call this directly when creating a processor instance,
    /// and instead will want to use concrete sound input types
    /// which call this method internally as needed.
    // TODO: rename to make_sound_input_base?
    pub fn make_sound_input(
        &mut self,
        options: InputOptions,
        branches: Vec<SoundInputBranchId>,
    ) -> ProcessorInput {
        ProcessorInput::new(self.input_idgen.next_id(), options, branches)
    }

    /// Internal helper method, only intended for use by ProcessorInput
    pub(super) fn make_input_argument(
        &mut self,
        instance: Rc<dyn AnySoundInputArgument>,
    ) -> SoundInputArgument {
        SoundInputArgument::new(self.input_arg_idgen.next_id(), instance)
    }

    /// Add an expression argument which reads a single scalar value
    /// from the processor's state using the given function.
    pub fn make_processor_scalar_argument(
        &mut self,
        function: ScalarReadFunc,
    ) -> ProcessorArgument {
        ProcessorArgument::new(
            self.proc_arg_idgen.next_id(),
            Rc::new(ScalarProcessorExpressionArgument::new(function)),
        )
    }

    /// Add an expression argument which reads an entire array from the
    /// sound processor's state using the provided function.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn make_processor_array_argument(&mut self, function: ArrayReadFunc) -> ProcessorArgument {
        ProcessorArgument::new(
            self.proc_arg_idgen.next_id(),
            Rc::new(ArrayProcessorExpressionArgument::new(function)),
        )
    }

    /// Add an expression argument which reads an entire array that
    /// is local to the sound processor's audio processing
    /// routine and must be provided with Context::push_processor_state.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn make_local_array_argument(&mut self) -> ProcessorArgument {
        ProcessorArgument::new(
            self.proc_arg_idgen.next_id(),
            Rc::new(ProcessorLocalArrayExpressionArgument::new()),
        )
    }

    /// Add an expression to the sound processor.
    /// When compiled, that expression can be
    /// executed directly on the audio thread
    /// to compute the result. See `make_expression_nodes`
    pub fn make_expression(
        &mut self,
        default_value: f32,
        scope: SoundExpressionScope,
    ) -> ProcessorExpression {
        ProcessorExpression::new(self.expression_idgen.next_id(), scope, default_value)
    }
}
