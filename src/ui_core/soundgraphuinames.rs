use eframe::epaint::ahash::{HashMap, HashMapExt};

use crate::core::sound::{
    expression::{ProcessorExpression, ProcessorExpressionLocation},
    expressionargument::{
        ArgumentLocation, ProcessorArgument, ProcessorArgumentLocation, SoundInputArgument,
        SoundInputArgumentLocation,
    },
    soundgraph::SoundGraph,
    soundinput::{BasicProcessorInput, ProcessorInputId, SoundInputLocation},
    soundprocessor::{ProcessorComponentVisitor, SoundProcessorId},
};

pub(crate) struct SoundArgumentNameData {
    name: String,
    location: ArgumentLocation,
}

impl SoundArgumentNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn location(&self) -> ArgumentLocation {
        self.location
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
    location: SoundInputLocation,
}

impl SoundInputNameData {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn location(&self) -> SoundInputLocation {
        self.location
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
    arguments: HashMap<ArgumentLocation, SoundArgumentNameData>,
    expressions: HashMap<ProcessorExpressionLocation, SoundExpressionNameData>,
    sound_inputs: HashMap<SoundInputLocation, SoundInputNameData>,
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
        self.arguments.retain(|k, _v| graph.contains(k));
        self.expressions.retain(|k, _v| graph.contains(k));
        self.sound_inputs.retain(|k, _v| graph.contains(k));
        self.sound_processors.retain(|k, _v| graph.contains(k));

        struct DefaultNameVisitor<'a> {
            names: &'a mut SoundGraphUiNames,
            processor_id: SoundProcessorId,
        }

        impl<'a> ProcessorComponentVisitor for DefaultNameVisitor<'a> {
            fn input(&mut self, input: &BasicProcessorInput) {
                let location = SoundInputLocation::new(self.processor_id, input.id());
                self.names
                    .sound_inputs
                    .entry(location)
                    .or_insert_with(|| SoundInputNameData {
                        name: format!("input_{}", location.input().value()),
                        location,
                    });
            }

            fn expression(&mut self, expression: &ProcessorExpression) {
                let location = ProcessorExpressionLocation::new(self.processor_id, expression.id());
                self.names
                    .expressions
                    .entry(location)
                    .or_insert_with(|| SoundExpressionNameData {
                        name: format!("expression_{}", location.expression().value()),
                    });
            }

            fn processor_argument(&mut self, argument: &ProcessorArgument) {
                let location = ProcessorArgumentLocation::new(self.processor_id, argument.id());
                self.names
                    .arguments
                    .entry(location.into())
                    .or_insert_with(|| SoundArgumentNameData {
                        name: format!("argument_{}", location.argument().value()),
                        location: location.into(),
                    });
            }

            fn input_argument(
                &mut self,
                argument: &SoundInputArgument,
                input_id: ProcessorInputId,
            ) {
                let location =
                    SoundInputArgumentLocation::new(self.processor_id, input_id, argument.id());
                self.names
                    .arguments
                    .entry(location.into())
                    .or_insert_with(|| SoundArgumentNameData {
                        name: format!("argument_{}", location.argument().value()),
                        location: location.into(),
                    });
            }
        }

        for proc_data in graph.sound_processors().values() {
            let mut visitor = DefaultNameVisitor {
                names: self,
                processor_id: proc_data.id(),
            };
            proc_data.instance().visit(&mut visitor);
        }
    }

    pub(crate) fn argument(&self, location: ArgumentLocation) -> Option<&SoundArgumentNameData> {
        self.arguments.get(&location)
    }

    pub(crate) fn expression(
        &self,
        location: ProcessorExpressionLocation,
    ) -> Option<&SoundExpressionNameData> {
        self.expressions.get(&location)
    }

    pub(crate) fn sound_input(&self, location: SoundInputLocation) -> Option<&SoundInputNameData> {
        self.sound_inputs.get(&location)
    }

    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&SoundProcessorNameData> {
        self.sound_processors.get(&id)
    }

    pub(crate) fn record_argument_name(&mut self, location: ArgumentLocation, name: String) {
        self.arguments.get_mut(&location).unwrap().name = name;
    }

    pub(crate) fn record_sound_input_name(&mut self, location: SoundInputLocation, name: String) {
        self.sound_inputs.get_mut(&location).unwrap().name = name;
    }

    pub(crate) fn record_sound_processor_name(&mut self, id: SoundProcessorId, name: String) {
        self.sound_processors.get_mut(&id).unwrap().name = name;
    }

    pub(crate) fn record_expression_name(&mut self, id: ProcessorExpressionLocation, name: String) {
        self.expressions.get_mut(&id).unwrap().name = name;
    }

    pub(crate) fn combined_parameter_name(&self, location: ArgumentLocation) -> String {
        match location {
            ArgumentLocation::Processor(location) => {
                format!(
                    "{}.{}",
                    self.sound_processor(location.processor()).unwrap().name(),
                    self.argument(location.into()).unwrap().name()
                )
            }
            ArgumentLocation::Input(location) => {
                format!(
                    "{}.{}.{}",
                    self.sound_processor(location.processor()).unwrap().name(),
                    self.sound_input(SoundInputLocation::new(
                        location.processor(),
                        location.input()
                    ))
                    .unwrap()
                    .name(),
                    self.argument(location.into()).unwrap().name()
                )
            }
        }
    }
}
