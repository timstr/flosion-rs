use crate::core::sound::{soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId};

use super::stategraphnode::{NodeTargetValue, UniqueProcessorNode};

pub struct NodeGen<'a, 'ctx> {
    topology: &'a SoundGraphTopology,
    inkwell_context: &'ctx inkwell::context::Context,
}

impl<'a, 'ctx> NodeGen<'a, 'ctx> {
    pub(crate) fn new(
        topology: &'a SoundGraphTopology,
        inkwell_context: &'ctx inkwell::context::Context,
    ) -> NodeGen<'a, 'ctx> {
        NodeGen {
            topology,
            inkwell_context,
        }
    }

    pub(crate) fn inkwell_context(&self) -> &'ctx inkwell::context::Context {
        self.inkwell_context
    }

    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        self.topology
    }

    pub(crate) fn allocate_sound_input_node(
        &self,
        sound_input_id: SoundInputId,
    ) -> NodeTargetValue<'ctx> {
        let input_data = self.topology.sound_input(sound_input_id).unwrap();
        match input_data.target() {
            Some(spid) => {
                let target_data = self.topology.sound_processor(spid).unwrap();
                let target_node =
                    UniqueProcessorNode::new(target_data.instance_arc().make_node(self));
                NodeTargetValue::Unique(target_node)
            }
            None => NodeTargetValue::Empty,
        }
    }
}
