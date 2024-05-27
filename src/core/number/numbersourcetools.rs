use super::{
    numbergraph::NumberGraphIdGenerators, numbergraphdata::NumberInputData,
    numbergraphtopology::NumberGraphTopology, numberinput::NumberInputHandle,
    numbersource::NumberSourceId,
};

pub struct NumberSourceTools<'a> {
    number_source_id: NumberSourceId,
    topology: &'a mut NumberGraphTopology,
    id_generators: &'a mut NumberGraphIdGenerators,
}

impl<'a> NumberSourceTools<'a> {
    pub(crate) fn new(
        number_source_id: NumberSourceId,
        topology: &'a mut NumberGraphTopology,
        id_generators: &'a mut NumberGraphIdGenerators,
    ) -> NumberSourceTools<'a> {
        NumberSourceTools {
            number_source_id,
            topology,
            id_generators,
        }
    }

    pub fn add_number_input(&mut self, default_value: f32) -> NumberInputHandle {
        let id = self.id_generators.number_input.next_id();
        let owner = self.number_source_id;
        let data = NumberInputData::new(id, owner, default_value);
        self.topology.add_number_input(data).unwrap();
        NumberInputHandle::new(id, owner)
    }

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) {
        self.topology.remove_number_input(handle.id()).unwrap();
    }
}
