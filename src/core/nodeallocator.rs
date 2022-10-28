use super::{
    soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
    soundprocessor::SoundProcessorId, statetree::ProcessorNodeWrapper,
};

pub struct NodeAllocator<'a> {
    processor_id: SoundProcessorId,
    topology: &'a SoundGraphTopology,
}

impl<'a> NodeAllocator<'a> {
    pub fn new(
        processor_id: SoundProcessorId,
        topology: &'a SoundGraphTopology,
    ) -> NodeAllocator<'a> {
        NodeAllocator {
            processor_id,
            topology,
        }
    }

    pub fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub fn make_state_tree_for(
        &self,
        input_id: SoundInputId,
    ) -> Option<Box<dyn ProcessorNodeWrapper>> {
        self.topology.make_state_tree_for(input_id)
    }
}
