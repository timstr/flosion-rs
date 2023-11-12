use std::slice;

use crate::core::{
    engine::{
        garbage::{Garbage, GarbageChute},
        nodegen::NodeGen,
    },
    jit::compilednumberinput::CompiledNumberInputFunction,
    sound::{context::Context, soundnumberinput::SoundNumberInputId},
};

pub struct SoundNumberInputNode<'ctx> {
    id: SoundNumberInputId,
    function: CompiledNumberInputFunction<'ctx>,
}

impl<'ctx> SoundNumberInputNode<'ctx> {
    pub(crate) fn new<'a>(
        id: SoundNumberInputId,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> SoundNumberInputNode<'ctx> {
        let function = nodegen.get_compiled_number_input(id);
        SoundNumberInputNode { id, function }
    }

    pub(crate) fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub(crate) fn update(
        &mut self,
        function: CompiledNumberInputFunction<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        let old_function = std::mem::replace(&mut self.function, function);
        old_function.toss(garbage_chute);
    }

    pub fn eval(&self, dst: &mut [f32], context: &Context) {
        self.function.eval(dst, context)
    }

    pub fn eval_scalar(&self, context: &Context) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        self.eval(s, context);
        s[0]
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
