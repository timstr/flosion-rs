use super::{soundinput::SoundInputId, stategraphnode::NodeTarget};

pub trait SoundInputNodeVisitor {
    fn visit_input(&mut self, input_id: SoundInputId, key_index: usize, target: &NodeTarget);
}

impl<F: FnMut(SoundInputId, usize, &NodeTarget)> SoundInputNodeVisitor for F {
    fn visit_input(&mut self, input_id: SoundInputId, key_index: usize, target: &NodeTarget) {
        (*self)(input_id, key_index, target);
    }
}

pub trait SoundInputNodeVisitorMut {
    fn visit_input(&mut self, input_id: SoundInputId, key_index: usize, target: &mut NodeTarget);
}

impl<F: FnMut(SoundInputId, usize, &mut NodeTarget)> SoundInputNodeVisitorMut for F {
    fn visit_input(&mut self, input_id: SoundInputId, key_index: usize, target: &mut NodeTarget) {
        (*self)(input_id, key_index, target);
    }
}

// Trait used for automating allocation and reallocation of node inputs
// Not concerned with actual audio processing or providing access to
// said inputs - concrete types will provide those.
pub trait SoundInputNode {
    fn flag_for_reset(&mut self);

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor);
    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut);

    fn add_input(&mut self, _input_id: SoundInputId);

    fn remove_input(&mut self, _input_id: SoundInputId);

    fn add_key(&mut self, _input_id: SoundInputId, _index: usize) {
        panic!("This input node type does not support keys");
    }

    fn remove_key(&mut self, _input_id: SoundInputId, _index: usize) {
        panic!("This input node type does not support keys");
    }
}

impl SoundInputNode for () {
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
    type NodeType: SoundInputNode;

    fn make_node(&self) -> Self::NodeType;
}
