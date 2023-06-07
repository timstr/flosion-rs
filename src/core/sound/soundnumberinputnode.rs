use std::slice;

use crate::core::{
    jit::compilednumberinput::CompiledNumberInputNode, sound::soundnumberinput::SoundNumberInputId,
};

use super::{context::Context, soundgraphtopology::SoundGraphTopology};

pub struct SoundNumberInputNode<'ctx> {
    id: SoundNumberInputId,
    artefact: Option<CompiledNumberInputNode<'ctx>>,
}

impl<'ctx> SoundNumberInputNode<'ctx> {
    pub(super) fn new(id: SoundNumberInputId) -> Self {
        Self { id, artefact: None }
    }

    pub(super) fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub(super) fn is_initialized(&self) -> bool {
        self.artefact.is_some()
    }

    pub(super) fn recompile(
        &mut self,
        topology: &SoundGraphTopology,
        inkwell_context: &'ctx inkwell::context::Context,
    ) {
        // TODO: skip recompilation if up to date
        self.artefact = Some(CompiledNumberInputNode::compile(
            self.id,
            topology,
            inkwell_context,
        ))
    }

    pub fn eval(&self, dst: &mut [f32], context: &Context) {
        match &self.artefact {
            Some(a) => a.eval(dst, context),
            None => panic!("Attempted to evaluate an unitialized NumberInputNode"),
        }
    }

    pub fn eval_scalar(&self, context: &Context) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        self.eval(s, context);
        s[0]
    }
}

pub trait SoundNumberInputNodeCollection<'ctx> {
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
