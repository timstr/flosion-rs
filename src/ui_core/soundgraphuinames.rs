use eframe::epaint::ahash::{HashMap, HashMapExt};

use crate::core::sound::{
    expression::SoundExpressionId,
    expressionargument::{SoundExpressionArgumentId, SoundExpressionArgumentOwner},
    soundgraph::SoundGraph,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

pub(crate) struct SoundArgumentNameData {
    name: String,
    owner: SoundExpressionArgumentOwner,
}

impl SoundArgumentNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn owner(&self) -> SoundExpressionArgumentOwner {
        self.owner
    }
}

pub(crate) struct SoundExpressionNameData {
    name: String,
}

impl SoundExpressionNameData {
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
    arguments: HashMap<SoundExpressionArgumentId, SoundArgumentNameData>,
    expressions: HashMap<SoundExpressionId, SoundExpressionNameData>,
    sound_inputs: HashMap<SoundInputId, SoundInputNameData>,
    sound_processors: HashMap<SoundProcessorId, SoundProcessorNameData>,
}

impl SoundGraphUiNames {
    pub(crate) fn new() -> SoundGraphUiNames {
        SoundGraphUiNames {
            arguments: HashMap::new(),
            expressions: HashMap::new(),
            sound_inputs: HashMap::new(),
            sound_processors: HashMap::new(),
        }
    }

    pub(crate) fn regenerate(&mut self, topology: &SoundGraph) {
        self.arguments
            .retain(|k, _v| topology.expression_argument(*k).is_some());
        self.expressions
            .retain(|k, _v| topology.expression(*k).is_some());
        self.sound_inputs
            .retain(|k, _v| topology.sound_input(*k).is_some());
        self.sound_processors
            .retain(|k, _v| topology.sound_processor(*k).is_some());

        for ns_data in topology.expression_arguments().values() {
            self.arguments
                .entry(ns_data.id())
                .or_insert_with(|| SoundArgumentNameData {
                    name: format!("argument_{}", ns_data.id().value()),
                    owner: ns_data.owner(),
                });
        }

        for ni_data in topology.expressions().values() {
            self.expressions
                .entry(ni_data.id())
                .or_insert_with(|| SoundExpressionNameData {
                    name: format!("expression_{}", ni_data.id().value()),
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
                        .instance_rc()
                        .as_graph_object()
                        .get_type()
                        .name()
                        .to_string(),
                });
        }
    }

    pub(crate) fn argument(&self, id: SoundExpressionArgumentId) -> Option<&SoundArgumentNameData> {
        self.arguments.get(&id)
    }

    pub(crate) fn expression(&self, id: SoundExpressionId) -> Option<&SoundExpressionNameData> {
        self.expressions.get(&id)
    }

    pub(crate) fn sound_input(&self, id: SoundInputId) -> Option<&SoundInputNameData> {
        self.sound_inputs.get(&id)
    }

    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&SoundProcessorNameData> {
        self.sound_processors.get(&id)
    }

    pub(crate) fn record_argument_name(&mut self, id: SoundExpressionArgumentId, name: String) {
        self.arguments.get_mut(&id).unwrap().name = name;
    }

    pub(crate) fn record_sound_input_name(&mut self, id: SoundInputId, name: String) {
        self.sound_inputs.get_mut(&id).unwrap().name = name;
    }

    pub(crate) fn record_sound_processor_name(&mut self, id: SoundProcessorId, name: String) {
        self.sound_processors.get_mut(&id).unwrap().name = name;
    }

    pub(crate) fn record_expression_name(&mut self, id: SoundExpressionId, name: String) {
        self.expressions.get_mut(&id).unwrap().name = name;
    }

    pub(crate) fn combined_parameter_name(&self, id: SoundExpressionArgumentId) -> String {
        let ns_data = self.argument(id).unwrap();
        match ns_data.owner() {
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                let sp_name = self.sound_processor(spid).unwrap().name();
                format!("{}.{}", sp_name, ns_data.name())
            }
            SoundExpressionArgumentOwner::SoundInput(siid) => {
                let si_data = self.sound_input(siid).unwrap();
                let sp_name = self.sound_processor(si_data.owner()).unwrap().name();
                format!("{}.{}.{}", sp_name, si_data.name(), ns_data.name())
            }
        }
    }
}
