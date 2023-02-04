use std::slice;

use super::{
    compilednumberinput::CompiledNumberInputNode, context::Context, numberinput::NumberInputId,
    soundgraphtopology::SoundGraphTopology,
};

pub struct NumberInputNode<'ctx> {
    id: NumberInputId,
    artefact: Option<CompiledNumberInputNode<'ctx>>,
}

impl<'ctx> NumberInputNode<'ctx> {
    pub(super) fn new(id: NumberInputId) -> Self {
        Self { id, artefact: None }
    }

    pub(super) fn id(&self) -> NumberInputId {
        self.id
    }

    pub(super) fn clear(&mut self) {
        self.artefact = None;
    }

    pub(super) fn recompile(
        &mut self,
        topology: &SoundGraphTopology,
        inkwell_context: &inkwell::context::Context,
    ) {
        if self.artefact.is_some() {
            return;
        }
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

pub trait NumberInputNodeCollection {
    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor);
    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut);

    fn add_input(&self, _input_id: NumberInputId) {
        panic!("This NumberInputNodeCollection type does not support adding inputs");
    }
    fn remove_input(&self, _input_id: NumberInputId) {
        panic!("This NumberInputNodeCollection type does not support removing inputs");
    }
}

pub trait NumberInputNodeVisitor {
    fn visit_node(&mut self, node: &NumberInputNode);
}

pub trait NumberInputNodeVisitorMut {
    fn visit_node(&mut self, node: &mut NumberInputNode);
}

impl<F: FnMut(&NumberInputNode)> NumberInputNodeVisitor for F {
    fn visit_node(&mut self, node: &NumberInputNode) {
        (*self)(node);
    }
}

impl<F: FnMut(&mut NumberInputNode)> NumberInputNodeVisitorMut for F {
    fn visit_node(&mut self, node: &mut NumberInputNode) {
        (*self)(node);
    }
}

impl NumberInputNodeCollection for () {
    fn visit_number_inputs(&self, _visitor: &mut dyn NumberInputNodeVisitor) {
        // Nothing to do
    }

    fn visit_number_inputs_mut(&mut self, _visitor: &mut dyn NumberInputNodeVisitorMut) {
        // Nothing to do
    }
}
