use crate::core::{
    engine::{
        nodegen::NodeGen,
        stategraphnode::{NodeTarget, NodeTargetValue},
    },
    sound::soundinput::SoundInputId,
};

// Trait used for automating allocation and reallocation of node inputs
// Not concerned with actual audio processing or providing access to
// said inputs - concrete types will provide those.
pub trait SoundInputNode<'ctx>: Sync + Send {
    fn targets(&self) -> &[NodeTarget<'ctx>];
    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>];

    fn insert_target(
        &mut self,
        _input_id: SoundInputId,
        _key_index: usize,
        _target: NodeTargetValue<'ctx>,
    ) {
        panic!("This input node type does not support inserting targets");
    }

    fn erase_target(
        &mut self,
        _input_id: SoundInputId,
        _key_index: usize,
    ) -> NodeTargetValue<'ctx> {
        panic!("This input node type does not support erasing targets");
    }
}

impl<'ctx> SoundInputNode<'ctx> for () {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        &[]
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        &mut []
    }
}

pub trait SoundProcessorInput: Sync + Send {
    type NodeType<'ctx>: SoundInputNode<'ctx>;

    fn make_node<'a, 'ctx>(&self, nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx>;

    fn list_ids(&self) -> Vec<SoundInputId>;
}
