use eframe::epaint::ahash::{HashMap, HashMapExt};

use crate::core::{
    sound::{
        soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId,
        soundnumberinput::SoundNumberInputId,
        soundnumbersource::{SoundNumberSourceId, SoundNumberSourceOwner},
        soundprocessor::SoundProcessorId,
    },
    uniqueid::UniqueId,
};

pub(crate) struct SoundNumberSourceNameData {
    name: String,
    owner: SoundNumberSourceOwner,
}

impl SoundNumberSourceNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn owner(&self) -> SoundNumberSourceOwner {
        self.owner
    }
}

pub(crate) struct SoundNumberInputNameData {
    name: String,
}

impl SoundNumberInputNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

pub(crate) struct SoundInputNameData {
    name: String,
    owner: SoundProcessorId,
}

impl SoundInputNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
    }
}

pub(crate) struct SoundProcessorNameData {
    name: String,
}

impl SoundProcessorNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

pub(crate) struct SoundGraphUiNames {
    number_sources: HashMap<SoundNumberSourceId, SoundNumberSourceNameData>,
    number_inputs: HashMap<SoundNumberInputId, SoundNumberInputNameData>,
    sound_inputs: HashMap<SoundInputId, SoundInputNameData>,
    sound_processors: HashMap<SoundProcessorId, SoundProcessorNameData>,
}

impl SoundGraphUiNames {
    pub(crate) fn new() -> SoundGraphUiNames {
        SoundGraphUiNames {
            number_sources: HashMap::new(),
            number_inputs: HashMap::new(),
            sound_inputs: HashMap::new(),
            sound_processors: HashMap::new(),
        }
    }

    pub(crate) fn regenerate(&mut self, topology: &SoundGraphTopology) {
        self.number_sources
            .retain(|k, _v| topology.number_source(*k).is_some());
        self.number_inputs
            .retain(|k, _v| topology.number_input(*k).is_some());
        self.sound_inputs
            .retain(|k, _v| topology.sound_input(*k).is_some());
        self.sound_processors
            .retain(|k, _v| topology.sound_processor(*k).is_some());

        for ns_data in topology.number_sources().values() {
            self.number_sources
                .entry(ns_data.id())
                .or_insert_with(|| SoundNumberSourceNameData {
                    name: format!("number_source_{}", ns_data.id().value()),
                    owner: ns_data.owner(),
                });
        }

        for ni_data in topology.number_inputs().values() {
            self.number_inputs
                .entry(ni_data.id())
                .or_insert_with(|| SoundNumberInputNameData {
                    name: format!("number_input_{}", ni_data.id().value()),
                });
        }

        for si_data in topology.sound_inputs().values() {
            self.sound_inputs
                .entry(si_data.id())
                .or_insert_with(|| SoundInputNameData {
                    name: format!("sound_input_{}", si_data.id().value()),
                    owner: si_data.owner(),
                });
        }

        for sp_data in topology.sound_processors().values() {
            self.sound_processors
                .entry(sp_data.id())
                .or_insert_with(|| SoundProcessorNameData {
                    name: sp_data
                        .instance_arc()
                        .as_graph_object()
                        .get_type()
                        .name()
                        .to_string(),
                });
        }
    }

    pub(crate) fn number_source(
        &self,
        id: SoundNumberSourceId,
    ) -> Option<&SoundNumberSourceNameData> {
        self.number_sources.get(&id)
    }

    pub(crate) fn number_input(&self, id: SoundNumberInputId) -> Option<&SoundNumberInputNameData> {
        self.number_inputs.get(&id)
    }

    pub(crate) fn sound_input(&self, id: SoundInputId) -> Option<&SoundInputNameData> {
        self.sound_inputs.get(&id)
    }

    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&SoundProcessorNameData> {
        self.sound_processors.get(&id)
    }

    pub(crate) fn record_number_source_name(&mut self, id: SoundNumberSourceId, name: &str) {
        self.number_sources.get_mut(&id).unwrap().name = name.to_string();
    }

    pub(crate) fn record_sound_input_name(&mut self, id: SoundInputId, name: &str) {
        self.sound_inputs.get_mut(&id).unwrap().name = name.to_string();
    }

    pub(crate) fn record_number_input_name(&mut self, id: SoundNumberInputId, name: &str) {
        self.number_inputs.get_mut(&id).unwrap().name = name.to_string();
    }

    pub(crate) fn combined_number_source_name(&self, id: SoundNumberSourceId) -> String {
        let ns_data = self.number_source(id).unwrap();
        match ns_data.owner() {
            SoundNumberSourceOwner::SoundProcessor(spid) => {
                let sp_name = self.sound_processor(spid).unwrap().name();
                format!("{}.{}", sp_name, ns_data.name())
            }
            SoundNumberSourceOwner::SoundInput(siid) => {
                let si_data = self.sound_input(siid).unwrap();
                let sp_name = self.sound_processor(si_data.owner()).unwrap().name();
                format!("{}.{}.{}", sp_name, si_data.name(), ns_data.name())
            }
        }
    }
}
