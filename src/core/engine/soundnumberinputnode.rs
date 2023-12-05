use std::slice;

use crate::core::{
    engine::{
        garbage::{Garbage, GarbageChute},
        nodegen::NodeGen,
    },
    jit::compilednumberinput::{CompiledNumberInputFunction, Discretization},
    sound::{
        context::Context, soundgraphdata::SoundNumberInputScope,
        soundnumberinput::SoundNumberInputId,
    },
};

pub struct SoundNumberInputNode<'ctx> {
    id: SoundNumberInputId,
    function: CompiledNumberInputFunction<'ctx>,

    #[cfg(debug_assertions)]
    scope: SoundNumberInputScope,
}

impl<'ctx> SoundNumberInputNode<'ctx> {
    #[cfg(not(debug_assertions))]
    pub(crate) fn new<'a>(
        id: SoundNumberInputId,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> SoundNumberInputNode<'ctx> {
        let function = nodegen.get_compiled_number_input(id);
        SoundNumberInputNode { id, function }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn new<'a>(
        id: SoundNumberInputId,
        nodegen: &NodeGen<'a, 'ctx>,
        scope: SoundNumberInputScope,
    ) -> SoundNumberInputNode<'ctx> {
        let function = nodegen.get_compiled_number_input(id);
        SoundNumberInputNode {
            id,
            function,
            scope,
        }
    }

    pub(crate) fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub(crate) fn reset(&mut self) {
        self.function.reset();
    }

    pub(crate) fn update(
        &mut self,
        function: CompiledNumberInputFunction<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        let old_function = std::mem::replace(&mut self.function, function);
        old_function.toss(garbage_chute);
    }

    pub fn eval(&mut self, dst: &mut [f32], discretization: Discretization, context: &Context) {
        self.function.eval(dst, context, discretization)
    }

    pub fn eval_scalar(&mut self, context: &Context) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        self.eval(s, Discretization::None, context);
        s[0]
    }

    #[cfg(debug_assertions)]
    pub(crate) fn scope(&self) -> &SoundNumberInputScope {
        &self.scope
    }
}

pub trait SoundNumberInputNodeCollection<'ctx>: Sync + Send {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>);
    fn visit_number_inputs_mut(
        &mut self,
        visitor: &'_ mut dyn SoundNumberInputNodeVisitorMut<'ctx>,
    );

    fn add_input(&self, _input_id: SoundNumberInputId) {
        panic!("This SoundNumberInputNodeCollection type does not support adding inputs");
    }
    fn remove_input(&self, _input_id: SoundNumberInputId) {
        panic!("This SoundNumberInputNodeCollection type does not support removing inputs");
    }
}

pub trait SoundNumberInputNodeVisitor<'ctx> {
    fn visit_node(&mut self, node: &SoundNumberInputNode<'ctx>);
}

pub trait SoundNumberInputNodeVisitorMut<'ctx> {
    fn visit_node(&mut self, node: &mut SoundNumberInputNode<'ctx>);
}

impl<'ctx, F: FnMut(&SoundNumberInputNode<'ctx>)> SoundNumberInputNodeVisitor<'ctx> for F {
    fn visit_node(&mut self, node: &SoundNumberInputNode<'ctx>) {
        (*self)(node);
    }
}

impl<'ctx, F: FnMut(&mut SoundNumberInputNode<'ctx>)> SoundNumberInputNodeVisitorMut<'ctx> for F {
    fn visit_node(&mut self, node: &mut SoundNumberInputNode<'ctx>) {
        (*self)(node);
    }
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for () {
    fn visit_number_inputs(&self, _visitor: &mut dyn SoundNumberInputNodeVisitor) {
        // Nothing to do
    }

    fn visit_number_inputs_mut(
        &mut self,
        _visitor: &'_ mut dyn SoundNumberInputNodeVisitorMut<'ctx>,
    ) {
        // Nothing to do
    }
}
