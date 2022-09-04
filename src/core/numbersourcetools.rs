use super::{
    numberinput::{NumberInputHandle, NumberInputOwner},
    numbersource::NumberSourceId,
    soundgraphtopology::SoundGraphTopology,
};

pub struct NumberSourceTools<'a> {
    number_source_id: NumberSourceId,
    topology: &'a mut SoundGraphTopology,
}

impl<'a> NumberSourceTools<'a> {
    pub(super) fn new(
        number_source_id: NumberSourceId,
        topology: &'a mut SoundGraphTopology,
    ) -> NumberSourceTools<'a> {
        NumberSourceTools {
            number_source_id,
            topology,
        }
    }

    pub fn add_number_input(&mut self) -> NumberInputHandle {
        // TODO: default_value for number source inputs
        let default_value: f32 = 0.0;
        self.topology.add_number_input(
            NumberInputOwner::NumberSource(self.number_source_id),
            default_value,
        )
    }

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) {
        self.topology.remove_number_input(handle.id());
    }
}
