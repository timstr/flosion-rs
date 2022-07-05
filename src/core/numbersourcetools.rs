use super::{
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::NumberSourceId,
    soundgraphtopology::SoundGraphTopology,
    uniqueid::IdGenerator,
};

pub struct NumberSourceTools<'a> {
    number_source_id: NumberSourceId,
    topology: &'a mut SoundGraphTopology,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
}

impl<'a> NumberSourceTools<'a> {
    pub(super) fn new(
        number_source_id: NumberSourceId,
        topology: &'a mut SoundGraphTopology,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    ) -> NumberSourceTools<'a> {
        NumberSourceTools {
            number_source_id,
            topology,
            number_input_idgen,
        }
    }

    pub fn add_number_input(&mut self) -> NumberInputHandle {
        let input_id = self.number_input_idgen.next_id();
        let owner = NumberInputOwner::NumberSource(self.number_source_id);
        let handle = NumberInputHandle::new(input_id, owner);
        self.topology.add_number_input(handle.clone());
        handle
    }

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) {
        self.topology.remove_number_input(handle.id());
    }
}
