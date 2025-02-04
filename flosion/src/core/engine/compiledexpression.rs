use std::slice;

use crate::core::{
    engine::garbage::{Garbage, GarbageChute},
    expression::context::ExpressionContext,
    jit::compiledexpression::{CompiledExpressionFunction, Discretization},
    sound::{
        expression::ProcessorExpressionId,
        soundprocessor::{CompiledComponentVisitor, CompiledProcessorComponent, StartOver},
    },
};

#[cfg(debug_assertions)]
use crate::core::sound::argument::ArgumentScope;

/// A compiled expression and all the data needed to directly
/// execute on the audio thread.
pub struct CompiledExpression<'ctx> {
    /// The expression which the node corresponds to
    id: ProcessorExpressionId,

    /// The JIT-compiled function to be executed
    function: CompiledExpressionFunction<'ctx>,

    #[cfg(debug_assertions)]
    /// The expression's scope, for debug validation only
    scope: ArgumentScope,
}

impl<'ctx> CompiledExpression<'ctx> {
    /// Creates a new compiled expression
    #[cfg(not(debug_assertions))]
    pub(crate) fn new<'a>(
        id: ProcessorExpressionId,
        function: CompiledExpressionFunction<'ctx>,
    ) -> CompiledExpression<'ctx> {
        CompiledExpression { id, function }
    }

    /// Creates a new compiled expression
    #[cfg(debug_assertions)]
    pub(crate) fn new<'a>(
        id: ProcessorExpressionId,
        function: CompiledExpressionFunction<'ctx>,
        scope: ArgumentScope,
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
        dsts: &mut [&mut [f32]],
        discretization: Discretization,
        context: ExpressionContext,
    ) {
        #[cfg(debug_assertions)]
        self.validate_context(&context);

        self.function.eval(dsts, context, discretization)
    }

    pub fn eval_scalar(
        &mut self,
        discretization: Discretization,
        context: ExpressionContext,
    ) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        self.eval(&mut [s], discretization, context);
        s[0]
    }

    #[cfg(debug_assertions)]
    /// Test whether the provided context matches the scope that the expression expects
    pub(crate) fn validate_context(&self, context: &ExpressionContext) -> bool {
        use crate::core::sound::argument::ProcessorArgumentId;

        let all_args = context.argument_stack().all_arguments();
        let previous_args = context.audio_context().argument_stack().all_arguments();

        let newly_pushed_args: Vec<ProcessorArgumentId> = all_args
            .into_iter()
            .filter(|arg| !previous_args.contains(arg))
            .collect();

        let mut all_good = true;
        for arg in &newly_pushed_args {
            if !self.scope.arguments().contains(&arg) {
                println!(
                    "A value was pushed for argument {} which is not marked as being \
                    in scope.",
                    arg.value()
                );
                all_good = false;
            }
        }
        for arg in self.scope.arguments() {
            if newly_pushed_args.iter().find(|a| **a == *arg).is_none() {
                println!(
                    "No value was pushed for argument {}, which is marked as being in scope.",
                    arg.value()
                );
                all_good = false;
            }
        }
        all_good
    }
}

impl<'ctx> CompiledProcessorComponent for CompiledExpression<'ctx> {
    fn visit(&self, _visitor: &mut dyn CompiledComponentVisitor) {}
}

impl<'ctx> StartOver for CompiledExpression<'ctx> {
    fn start_over(&mut self) {
        CompiledExpression::start_over(self);
    }
}
