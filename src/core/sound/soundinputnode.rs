use crate::core::engine::stategraphnode::NodeTarget;

use super::soundinput::{InputTiming, SoundInputId};

pub trait SoundInputNodeVisitor<'ctx> {
    fn visit_input(&mut self, input_id: SoundInputId, key_index: usize, target: &NodeTarget<'ctx>);
}

impl<'ctx, F: FnMut(SoundInputId, usize, &NodeTarget<'ctx>)> SoundInputNodeVisitor<'ctx> for F {
    fn visit_input(&mut self, input_id: SoundInputId, key_index: usize, target: &NodeTarget<'ctx>) {
        (*self)(input_id, key_index, target);
    }
}

pub trait SoundInputNodeVisitorMut<'ctx> {
    fn visit_input(
        &mut self,
        input_id: SoundInputId,
        key_index: usize,
        target: &mut NodeTarget<'ctx>,
        timing: &mut InputTiming,
    );
}

impl<'ctx, F: FnMut(SoundInputId, usize, &mut NodeTarget<'ctx>, &mut InputTiming)>
    SoundInputNodeVisitorMut<'ctx> for F
{
    fn visit_input(
        &mut self,
        input_id: SoundInputId,
        key_index: usize,
        target: &mut NodeTarget<'ctx>,
        timing: &mut InputTiming,
    ) {
        (*self)(input_id, key_index, target, timing);
    }
}

// Trait used for automating allocation and reallocation of node inputs
// Not concerned with actual audio processing or providing access to
// said inputs - concrete types will provide those.
pub trait SoundInputNode<'ctx> {
    fn flag_for_reset(&mut self);

    fn visit_inputs<'a>(&self, visitor: &'a mut dyn SoundInputNodeVisitor<'ctx>);
    fn visit_inputs_mut<'a>(&mut self, visitor: &'a mut dyn SoundInputNodeVisitorMut<'ctx>);

    fn add_input(&mut self, _input_id: SoundInputId);

    fn remove_input(&mut self, _input_id: SoundInputId);

    fn add_key(&mut self, _input_id: SoundInputId, _index: usize) {
        panic!("This input node type does not support keys");
    }

    fn remove_key(&mut self, _input_id: SoundInputId, _index: usize) {
        panic!("This input node type does not support keys");
    }
}

impl<'ctx> SoundInputNode<'ctx> for () {
    fn flag_for_reset(&mut self) {}

    fn visit_inputs(&self, _visitor: &mut dyn SoundInputNodeVisitor) {}
    fn visit_inputs_mut(&mut self, _visitor: &mut dyn SoundInputNodeVisitorMut) {}

    fn add_input(&mut self, _input_id: SoundInputId) {
        panic!("This input node type does not support adding any inputs");
    }

    fn remove_input(&mut self, _input_id: SoundInputId) {
        panic!("This input node type does not support adding any inputs");
    }
}

pub trait SoundProcessorInput {
    type NodeType<'ctx>: SoundInputNode<'ctx>;

    fn make_node<'ctx>(&self) -> Self::NodeType<'ctx>;

    fn list_ids(&self) -> Vec<SoundInputId>;
}
