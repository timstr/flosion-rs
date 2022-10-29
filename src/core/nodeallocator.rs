use super::{
    soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
    soundprocessor::SoundProcessorId, statetree::ProcessorNodeWrapper,
};

// TODO: remove this in favour of visitor which has mutable access to allocate nodes in place
pub struct NodeAllocator<'a> {
    processor_id: SoundProcessorId,
    topology: &'a SoundGraphTopology,
}

impl<'a> NodeAllocator<'a> {
    pub(super) fn new(
        processor_id: SoundProcessorId,
        topology: &'a SoundGraphTopology,
    ) -> NodeAllocator<'a> {
        NodeAllocator {
            processor_id,
            topology,
        }
    }

    pub(super) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub(super) fn make_state_tree_for(
        &self,
        input_id: SoundInputId,
    ) -> Option<Box<dyn ProcessorNodeWrapper>> {
        self.topology.make_state_tree_for(input_id)
    }
}
