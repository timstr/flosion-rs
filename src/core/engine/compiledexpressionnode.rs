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

pub struct CompiledExpressionNode<'ctx> {
    id: SoundExpressionId,
    function: CompiledExpressionFunction<'ctx>,

    #[cfg(debug_assertions)]
    scope: SoundExpressionScope,
}

impl<'ctx> CompiledExpressionNode<'ctx> {
    #[cfg(not(debug_assertions))]
    pub(crate) fn new<'a>(
        id: SoundExpressionId,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> CompiledExpressionNode<'ctx> {
        let function = nodegen.get_compiled_expression(id);
        CompiledExpressionNode { id, function }
    }

    #[cfg(debug_assertions)]
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

    pub(crate) fn id(&self) -> SoundExpressionId {
        self.id
    }

    pub(crate) fn reset(&mut self) {
        self.function.reset();
    }

    pub(crate) fn update(
        &mut self,
        function: CompiledExpressionFunction<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        let old_function = std::mem::replace(&mut self.function, function);
        old_function.toss(garbage_chute);
    }

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

pub trait ExpressionCollection<'ctx>: Sync + Send {
    fn visit_expressions(&self, visitor: &mut dyn ExpressionVisitor<'ctx>);
    fn visit_expressions_mut(&mut self, visitor: &'_ mut dyn ExpressionVisitorMut<'ctx>);

    fn add_input(&self, _input_id: SoundExpressionId) {
        panic!("This ExpressionCollection type does not support adding expressions");
    }
    fn remove_input(&self, _input_id: SoundExpressionId) {
        panic!("This ExpressionCollection type does not support removing expressions");
    }
}

pub trait ExpressionVisitor<'ctx> {
    fn visit_node(&mut self, node: &CompiledExpressionNode<'ctx>);
}

pub trait ExpressionVisitorMut<'ctx> {
    fn visit_node(&mut self, node: &mut CompiledExpressionNode<'ctx>);
}

impl<'ctx, F: FnMut(&CompiledExpressionNode<'ctx>)> ExpressionVisitor<'ctx> for F {
    fn visit_node(&mut self, node: &CompiledExpressionNode<'ctx>) {
        (*self)(node);
    }
}

impl<'ctx, F: FnMut(&mut CompiledExpressionNode<'ctx>)> ExpressionVisitorMut<'ctx> for F {
    fn visit_node(&mut self, node: &mut CompiledExpressionNode<'ctx>) {
        (*self)(node);
    }
}

impl<'ctx> ExpressionCollection<'ctx> for () {
    fn visit_expressions(&self, _visitor: &mut dyn ExpressionVisitor) {
        // Nothing to do
    }

    fn visit_expressions_mut(&mut self, _visitor: &'_ mut dyn ExpressionVisitorMut<'ctx>) {
        // Nothing to do
    }
}
