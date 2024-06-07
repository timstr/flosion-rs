use std::slice;

use crate::core::{
    engine::{
        garbage::{Garbage, GarbageChute},
        nodegen::NodeGen,
    },
    jit::compiledexpression::{CompiledExpressionFunction, Discretization},
    sound::{context::Context, expression::SoundExpressionId},
};

#[cfg(debug_assertions)]
use crate::core::sound::soundgraphdata::SoundExpressionScope;

/// A compiled expression and all the data needed to directly
/// execute it within a StateGraph instance on the audio thread
pub struct CompiledExpressionNode<'ctx> {
    /// The expression which the node corresponds to
    id: SoundExpressionId,

    /// The JIT-compiled function to be executed
    function: CompiledExpressionFunction<'ctx>,

    #[cfg(debug_assertions)]
    /// The expression's scope, for debug validation only
    scope: SoundExpressionScope,
}

impl<'ctx> CompiledExpressionNode<'ctx> {
    #[cfg(not(debug_assertions))]
    /// Creates a new compiled expression node
    pub(crate) fn new<'a>(
        id: SoundExpressionId,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> CompiledExpressionNode<'ctx> {
        let function = nodegen.get_compiled_expression(id);
        CompiledExpressionNode { id, function }
    }

    #[cfg(debug_assertions)]
    /// Creates a new compiled expression node
    pub(crate) fn new<'a>(
        id: SoundExpressionId,
        nodegen: &NodeGen<'a, 'ctx>,
        scope: SoundExpressionScope,
    ) -> CompiledExpressionNode<'ctx> {
        let function = nodegen.get_compiled_expression(id);
        CompiledExpressionNode {
            id,
            function,
            scope,
        }
    }

    /// Retrieve the id of the expression the node corresponds to
    pub(crate) fn id(&self) -> SoundExpressionId {
        self.id
    }

    /// Flag the compiled function to start over when it is next
    /// evaluated. This is a lightweight operation.
    // TODO: consider renaming 'reset' here and elsewhere since it makes
    // me think of e.g. resetting a C++ std::unique_ptr or other class
    // Maybe 'start_over'?
    pub(crate) fn reset(&mut self) {
        self.function.reset();
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
    pub fn eval(&mut self, dst: &mut [f32], discretization: Discretization, context: &Context) {
        #[cfg(debug_assertions)]
        self.validate_context(dst.len(), context);

        self.function.eval(dst, context, discretization)
    }

    pub fn eval_scalar(&mut self, context: &Context) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        self.eval(s, Discretization::None, context);
        s[0]
    }

    #[cfg(debug_assertions)]
    /// Test whether the provided context matches the scope that the expression expects
    pub(crate) fn validate_context(&self, expected_len: usize, context: &Context) -> bool {
        use crate::core::{sound::context::StackFrame, uniqueid::UniqueId};

        let stack = context.stack();
        let StackFrame::Processor(frame) = stack else {
            println!("Processor state must be pushed onto context when evaluating expression");
            return false;
        };
        let local_arrays = frame.local_arrays().as_vec();
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

/// Trait for describing and modifying the set of compiled expressions
/// belonging to a sound processor node in the state graph. The methods
/// visit_expressions and visit_expressions_mut are required for inspecting
/// and replacing the allocated nodes. The optional methods add_expression
/// and remove_expression are only needed if expressions can be added and
/// removed after the processor's construction.
pub trait ExpressionCollection<'ctx>: Sync + Send {
    fn visit_expressions(&self, visitor: &mut dyn ExpressionVisitor<'ctx>);
    fn visit_expressions_mut(&mut self, visitor: &'_ mut dyn ExpressionVisitorMut<'ctx>);

    fn add_expression(&self, _input_id: SoundExpressionId) {
        panic!("This ExpressionCollection type does not support adding expressions");
    }
    fn remove_expression(&self, _input_id: SoundExpressionId) {
        panic!("This ExpressionCollection type does not support removing expressions");
    }
}

/// A trait for inspecting each expression node in an ExpressionCollection
pub trait ExpressionVisitor<'ctx> {
    fn visit_node(&mut self, node: &CompiledExpressionNode<'ctx>);
}

/// A trait for modifying each expression node in an ExpressionCollection
pub trait ExpressionVisitorMut<'ctx> {
    fn visit_node(&mut self, node: &mut CompiledExpressionNode<'ctx>);
}

/// Blanket implementation of ExpressionVisitor for functions
impl<'ctx, F: FnMut(&CompiledExpressionNode<'ctx>)> ExpressionVisitor<'ctx> for F {
    fn visit_node(&mut self, node: &CompiledExpressionNode<'ctx>) {
        (*self)(node);
    }
}

/// Blanket implementation of ExpressionVisitorMut for functions
impl<'ctx, F: FnMut(&mut CompiledExpressionNode<'ctx>)> ExpressionVisitorMut<'ctx> for F {
    fn visit_node(&mut self, node: &mut CompiledExpressionNode<'ctx>) {
        (*self)(node);
    }
}

/// The unit type `()` can be used as an ExpressionCollection containing no expressions
impl<'ctx> ExpressionCollection<'ctx> for () {
    fn visit_expressions(&self, _visitor: &mut dyn ExpressionVisitor) {
        // Nothing to do
    }

    fn visit_expressions_mut(&mut self, _visitor: &'_ mut dyn ExpressionVisitorMut<'ctx>) {
        // Nothing to do
    }
}
