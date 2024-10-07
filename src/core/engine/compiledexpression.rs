use std::slice;

use crate::core::{
    engine::garbage::{Garbage, GarbageChute},
    expression::context::ExpressionContext,
    jit::compiledexpression::{CompiledExpressionFunction, Discretization},
    sound::{expression::ProcessorExpressionId, soundprocessor::CompiledProcessorComponent},
};

#[cfg(debug_assertions)]
use crate::core::sound::expression::SoundExpressionScope;

/// A compiled expression and all the data needed to directly
/// execute it within a StateGraph instance on the audio thread.
pub struct CompiledExpression<'ctx> {
    /// The expression which the node corresponds to
    id: ProcessorExpressionId,

    /// The JIT-compiled function to be executed
    function: CompiledExpressionFunction<'ctx>,

    #[cfg(debug_assertions)]
    /// The expression's scope, for debug validation only
    scope: SoundExpressionScope,
}

impl<'ctx> CompiledExpression<'ctx> {
    #[cfg(not(debug_assertions))]
    /// Creates a new compiled expression
    pub(crate) fn new<'a>(
        id: SoundExpressionId,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> CompiledExpression<'ctx> {
        let function = compiler.get_compiled_expression(id);
        CompiledExpression { id, function }
    }

    #[cfg(debug_assertions)]
    /// Creates a new compiled expression
    pub(crate) fn new<'a>(
        id: ProcessorExpressionId,
        function: CompiledExpressionFunction<'ctx>,
        scope: SoundExpressionScope,
    ) -> CompiledExpression<'ctx> {
        CompiledExpression {
            id,
            function,
            scope,
        }
    }

    /// Retrieve the id of the expression
    pub(crate) fn id(&self) -> ProcessorExpressionId {
        self.id
    }

    /// Flag the compiled function to start over when it is next
    /// evaluated. This is a lightweight operation.
    pub(crate) fn start_over(&mut self) {
        self.function.start_over();
    }

    /// Swap the existing compiled function for a new one, disposing of
    /// the existing one in the provided garbage chute.
    pub(crate) fn replace(
        &mut self,
        function: CompiledExpressionFunction<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        let old_function = std::mem::replace(&mut self.function, function);
        old_function.toss(garbage_chute);
    }

    /// Invoke the compiled function on the provided array. Individual array
    /// values are interpreted according to the given discretization, e.g.
    /// to correctly model how far apart adjacent array entries are in time
    pub fn eval(
        &mut self,
        dst: &mut [f32],
        discretization: Discretization,
        context: ExpressionContext,
    ) {
        #[cfg(debug_assertions)]
        self.validate_context(dst.len(), &context);

        self.function.eval(dst, context, discretization)
    }

    pub fn eval_scalar(
        &mut self,
        discretization: Discretization,
        context: ExpressionContext,
    ) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        self.eval(s, discretization, context);
        s[0]
    }

    #[cfg(debug_assertions)]
    /// Test whether the provided context matches the scope that the expression expects
    pub(crate) fn validate_context(
        &self,
        expected_len: usize,
        context: &ExpressionContext,
    ) -> bool {
        if self.scope.processor_state_available() {
            if context.top_processor_state().is_none() {
                println!("The processor state was marked as available but it was not provided");
                return false;
            }
        } else {
            if context.top_processor_state().is_some() {
                println!("The processor state was marked as unavailable but it was provided");
                return false;
            }
        }

        let local_arrays = context.top_processor_arrays().as_vec();
        for arr in &local_arrays {
            if !self
                .scope
                .available_local_arguments()
                .contains(&arr.argument_id())
            {
                println!(
                    "A local array was pushed for expression argument {} which is not marked as being \
                    in scope.",
                    arr.argument_id().value()
                );
                return false;
            }
            if arr.array().len() != expected_len {
                println!(
                    "A local array was pushed for expression argument {}, but its length of {} doesn't \
                    match the expected length from the destination array of {}.",
                    arr.argument_id().value(),
                    arr.array().len(),
                    expected_len
                );
                return false;
            }
        }
        for nsid in self.scope.available_local_arguments() {
            if local_arrays
                .iter()
                .find(|a| a.argument_id() == *nsid)
                .is_none()
            {
                println!(
                    "No local array was pushed for expression argument {}, which is marked as being in scope.",
                    nsid.value()
                );
                return false;
            }
        }
        true
    }
}

impl<'ctx> CompiledProcessorComponent<'ctx> for CompiledExpression<'ctx> {
    fn start_over(&mut self) {
        CompiledExpression::start_over(self);
    }
}
