use crate::core::{
    jit::{compilednumberinput::CompiledNumberInputFunction, server::JitServer},
    sound::{
        soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
        soundnumberinput::SoundNumberInputId,
    },
};

use super::stategraphnode::{NodeTargetValue, UniqueProcessorNode};

pub struct NodeGen<'a, 'ctx> {
    topology: &'a SoundGraphTopology,
    jit_server: &'a JitServer<'ctx>,
}

impl<'a, 'ctx> NodeGen<'a, 'ctx> {
    pub(crate) fn new(
        topology: &'a SoundGraphTopology,
        jit_server: &'a JitServer<'ctx>,
    ) -> NodeGen<'a, 'ctx> {
        NodeGen {
            topology,
            jit_server,
        }
    }

    pub(crate) fn get_compiled_number_input(
        &self,
        id: SoundNumberInputId,
    ) -> CompiledNumberInputFunction<'ctx> {
        self.jit_server.get_compiled_number_input(id, self.topology)
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
