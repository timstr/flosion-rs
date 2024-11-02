use std::collections::HashMap;

use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::sound::{
    argument::{AnyProcessorArgument, ProcessorArgumentLocation},
    expression::{ProcessorExpression, ProcessorExpressionLocation},
    soundgraph::SoundGraph,
    soundinput::{BasicProcessorInput, SoundInputLocation},
    soundprocessor::{ProcessorComponentVisitor, SoundProcessorId},
};

pub(crate) struct SoundGraphUiNames {
    arguments: HashMap<ProcessorArgumentLocation, String>,
    expressions: HashMap<ProcessorExpressionLocation, String>,
    sound_inputs: HashMap<SoundInputLocation, String>,
    sound_processors: HashMap<SoundProcessorId, String>,
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

    pub(crate) fn cleanup(&mut self, graph: &SoundGraph) {
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
                    .or_insert_with(|| format!("input_{}", location.input().value()));
            }

            fn expression(&mut self, expression: &ProcessorExpression) {
                let location = ProcessorExpressionLocation::new(self.processor_id, expression.id());
                self.names
                    .expressions
                    .entry(location)
                    .or_insert_with(|| format!("expression_{}", location.expression().value()));
            }

            fn argument(&mut self, argument: &dyn AnyProcessorArgument) {
                let location = ProcessorArgumentLocation::new(self.processor_id, argument.id());
                self.names
                    .arguments
                    .entry(location.into())
                    .or_insert_with(|| format!("argument_{}", location.argument().value()));
            }
        }

        for proc_data in graph.sound_processors().values() {
            self.sound_processors
                .entry(proc_data.id())
                .or_insert_with(|| proc_data.as_graph_object().friendly_name());
            let mut visitor = DefaultNameVisitor {
                names: self,
                processor_id: proc_data.id(),
            };
            proc_data.visit(&mut visitor);
        }
    }

    pub(crate) fn argument(&self, location: ProcessorArgumentLocation) -> Option<&str> {
        self.arguments.get(&location).map(|s| s.as_str())
    }

    pub(crate) fn expression(&self, location: ProcessorExpressionLocation) -> Option<&str> {
        self.expressions.get(&location).map(|s| s.as_str())
    }

    pub(crate) fn sound_input(&self, location: SoundInputLocation) -> Option<&str> {
        self.sound_inputs.get(&location).map(|s| s.as_str())
    }

    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&str> {
        self.sound_processors.get(&id).map(|s| s.as_str())
    }

    pub(crate) fn record_argument_name(
        &mut self,
        location: ProcessorArgumentLocation,
        name: String,
    ) {
        *self.arguments.get_mut(&location).unwrap() = name;
    }

    pub(crate) fn record_sound_input_name(&mut self, location: SoundInputLocation, name: String) {
        *self.sound_inputs.get_mut(&location).unwrap() = name;
    }

    pub(crate) fn record_sound_processor_name(&mut self, id: SoundProcessorId, name: String) {
        *self.sound_processors.get_mut(&id).unwrap() = name;
    }

    pub(crate) fn record_expression_name(&mut self, id: ProcessorExpressionLocation, name: String) {
        *self.expressions.get_mut(&id).unwrap() = name;
    }

    pub(crate) fn combined_parameter_name(&self, location: ProcessorArgumentLocation) -> String {
        format!(
            "{}.{}",
            self.sound_processor(location.processor()).unwrap(),
            self.argument(location.into()).unwrap()
        )
    }

    pub(crate) fn check_invariants(&self, graph: &SoundGraph) {
        // all named components exist in the graph
        for location in self.arguments.keys() {
            assert!(graph.contains(location));
        }
        for location in self.expressions.keys() {
            assert!(graph.contains(location));
        }
        for location in self.sound_inputs.keys() {
            assert!(graph.contains(location));
        }
        for location in self.sound_processors.keys() {
            assert!(graph.contains(location));
        }

        // all components of the graph are named
        for proc in graph.sound_processors().values() {
            assert!(self.sound_processors.contains_key(&proc.id()));

            proc.foreach_input(|_, location| {
                assert!(self.sound_inputs.contains_key(&location));
            });

            proc.foreach_expression(|_, location| {
                assert!(self.expressions.contains_key(&location));
            });

            proc.foreach_argument(|_, location| {
                assert!(self.arguments.contains_key(&location.into()));
            });
        }
    }
}

impl Stashable for SoundGraphUiNames {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.arguments.iter(),
            |(loc, name), stasher| {
                loc.stash(stasher);
                stasher.string(name);
            },
            Order::Unordered,
        );
        stasher.array_of_proxy_objects(
            self.expressions.iter(),
            |(loc, name), stasher| {
                loc.stash(stasher);
                stasher.string(name);
            },
            Order::Unordered,
        );
        stasher.array_of_proxy_objects(
            self.sound_inputs.iter(),
            |(loc, name), stasher| {
                loc.stash(stasher);
                stasher.string(name);
            },
            Order::Unordered,
        );
        stasher.array_of_proxy_objects(
            self.sound_processors.iter(),
            |(id, name), stasher| {
                id.stash(stasher);
                stasher.string(name);
            },
            Order::Unordered,
        );
    }
}

impl Unstashable for SoundGraphUiNames {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let mut arguments = HashMap::new();
        let mut expressions = HashMap::new();
        let mut sound_inputs = HashMap::new();
        let mut sound_processors = HashMap::new();

        unstasher.array_of_proxy_objects(|unstasher| {
            arguments.insert(
                ProcessorArgumentLocation::unstash(unstasher)?,
                unstasher.string()?,
            );
            Ok(())
        })?;

        unstasher.array_of_proxy_objects(|unstasher| {
            expressions.insert(
                ProcessorExpressionLocation::unstash(unstasher)?,
                unstasher.string()?,
            );
            Ok(())
        })?;

        unstasher.array_of_proxy_objects(|unstasher| {
            sound_inputs.insert(SoundInputLocation::unstash(unstasher)?, unstasher.string()?);
            Ok(())
        })?;

        unstasher.array_of_proxy_objects(|unstasher| {
            sound_processors.insert(SoundProcessorId::unstash(unstasher)?, unstasher.string()?);
            Ok(())
        })?;

        Ok(SoundGraphUiNames {
            arguments,
            expressions,
            sound_inputs,
            sound_processors,
        })
    }
}
