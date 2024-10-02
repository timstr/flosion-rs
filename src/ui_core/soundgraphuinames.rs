use eframe::epaint::ahash::{HashMap, HashMapExt};

use crate::core::sound::{
    expression::ProcessorExpressionLocation,
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
    expressions: HashMap<ProcessorExpressionLocation, SoundExpressionNameData>,
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

    pub(crate) fn regenerate(&mut self, graph: &SoundGraph) {
        self.arguments.retain(|k, _v| graph.contains(*k));
        self.expressions.retain(|k, _v| {
            // TODO: check expression exists also
            graph.contains(k.processor())
        });
        self.sound_inputs.retain(|k, _v| graph.contains(*k));
        self.sound_processors.retain(|k, _v| graph.contains(*k));

        for ns_data in graph.expression_arguments().values() {
            self.arguments
                .entry(ns_data.id())
                .or_insert_with(|| SoundArgumentNameData {
                    name: format!("argument_{}", ns_data.id().value()),
                    owner: ns_data.owner(),
                });
        }

        for si_data in graph.sound_inputs().values() {
            self.sound_inputs
                .entry(si_data.id())
                .or_insert_with(|| SoundInputNameData {
                    name: format!("sound_input_{}", si_data.id().value()),
                    owner: si_data.owner(),
                });
        }

        for sp_data in graph.sound_processors().values() {
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

            sp_data.foreach_expression(|expr| {
                let location = ProcessorExpressionLocation::new(sp_data.id(), expr.id());
                self.expressions
                    .entry(location)
                    .or_insert_with(|| SoundExpressionNameData {
                        name: format!("expression_{}", expr.id().value()),
                    });
            });
        }
    }

    pub(crate) fn argument(&self, id: SoundExpressionArgumentId) -> Option<&SoundArgumentNameData> {
        self.arguments.get(&id)
    }

    pub(crate) fn expression(
        &self,
        id: ProcessorExpressionLocation,
    ) -> Option<&SoundExpressionNameData> {
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

    pub(crate) fn record_expression_name(&mut self, id: ProcessorExpressionLocation, name: String) {
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
