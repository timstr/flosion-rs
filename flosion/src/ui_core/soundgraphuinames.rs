use std::collections::HashMap;

use hashstash::{Order, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::core::{
    expression::expressioninput::ExpressionInputId,
    sound::{
        argument::{AnyProcessorArgument, ProcessorArgumentLocation},
        expression::{ProcessorExpression, ProcessorExpressionLocation},
        soundgraph::SoundGraph,
        soundinput::{AnyProcessorInput, SoundInputLocation},
        soundprocessor::{ProcessorComponentVisitor, SoundProcessorId},
    },
};

pub(crate) struct SoundGraphUiNames {
    arguments: HashMap<ProcessorArgumentLocation, String>,
    expression_results: HashMap<(ProcessorExpressionLocation, ExpressionInputId), String>,
    sound_inputs: HashMap<SoundInputLocation, String>,
    sound_processors: HashMap<SoundProcessorId, String>,
}

impl SoundGraphUiNames {
    pub(crate) fn new() -> SoundGraphUiNames {
        SoundGraphUiNames {
            arguments: HashMap::new(),
            expression_results: HashMap::new(),
            sound_inputs: HashMap::new(),
            sound_processors: HashMap::new(),
        }
    }

    fn parse_next_highest_number<'a, I: Iterator<Item = &'a String>>(
        iter: I,
        prefix: &str,
    ) -> usize {
        let mut next_highest_number = 1;
        for name in iter {
            let Some(rest_of_name) = name.strip_prefix(prefix) else {
                continue;
            };
            if let Ok(i) = rest_of_name.parse::<usize>() {
                next_highest_number = next_highest_number.max(i + 1);
            }
        }
        next_highest_number
    }

    pub(crate) fn cleanup(&mut self, graph: &SoundGraph) {
        self.arguments.retain(|k, _v| graph.contains(k));
        self.expression_results.retain(|k, _v| graph.contains(k.0));
        self.sound_inputs.retain(|k, _v| graph.contains(k));
        self.sound_processors.retain(|k, _v| graph.contains(k));

        struct DefaultNameVisitor<'a> {
            names: &'a mut SoundGraphUiNames,
            processor_id: SoundProcessorId,
            next_argument_number: usize,
            next_sound_input_number: usize,
        }

        const PREFIX_ARGUMENT: &str = "argument";
        const PREFIX_INPUT: &str = "input";

        impl<'a> ProcessorComponentVisitor for DefaultNameVisitor<'a> {
            fn input(&mut self, input: &dyn AnyProcessorInput) {
                let location = SoundInputLocation::new(self.processor_id, input.id());
                self.names.sound_inputs.entry(location).or_insert_with(|| {
                    let i = self.next_sound_input_number;
                    self.next_sound_input_number += 1;
                    format!("{}{}", PREFIX_INPUT, i)
                });
            }

            fn expression(&mut self, expression: &ProcessorExpression) {
                let location = ProcessorExpressionLocation::new(self.processor_id, expression.id());
                for (i, result) in expression.graph().results().iter().enumerate() {
                    self.names
                        .expression_results
                        .entry((location, result.id()))
                        .or_insert_with(|| format!("result{}", i + 1));
                }
            }

            fn argument(&mut self, argument: &dyn AnyProcessorArgument) {
                let location = ProcessorArgumentLocation::new(self.processor_id, argument.id());
                self.names
                    .arguments
                    .entry(location.into())
                    .or_insert_with(|| {
                        let i = self.next_argument_number;
                        self.next_argument_number += 1;
                        format!("{}{}", PREFIX_ARGUMENT, i)
                    });
            }
        }

        for proc_data in graph.sound_processors().values() {
            let type_name = proc_data.as_graph_object().get_dynamic_type().name();
            let mut next_processor_number =
                Self::parse_next_highest_number(self.sound_processors.values(), type_name);
            self.sound_processors
                .entry(proc_data.id())
                .or_insert_with(|| {
                    let i = next_processor_number;
                    next_processor_number += 1;
                    format!("{}{}", type_name, i)
                });

            let next_argument_number =
                Self::parse_next_highest_number(self.arguments.values(), PREFIX_ARGUMENT);
            let next_sound_input_number =
                Self::parse_next_highest_number(self.sound_inputs.values(), PREFIX_INPUT);

            let mut visitor = DefaultNameVisitor {
                names: self,
                processor_id: proc_data.id(),
                next_argument_number,
                next_sound_input_number,
            };
            proc_data.visit(&mut visitor);
        }
    }

    pub(crate) fn argument(&self, location: ProcessorArgumentLocation) -> Option<&str> {
        self.arguments.get(&location).map(|s| s.as_str())
    }

    pub(crate) fn expression_result(
        &self,
        location: ProcessorExpressionLocation,
        result_id: ExpressionInputId,
    ) -> Option<&str> {
        self.expression_results
            .get(&(location, result_id))
            .map(|s| s.as_str())
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

    pub(crate) fn record_expression_result_name(
        &mut self,
        location: ProcessorExpressionLocation,
        result_id: ExpressionInputId,
        name: String,
    ) {
        *self
            .expression_results
            .get_mut(&(location, result_id))
            .unwrap() = name;
    }

    pub(crate) fn combined_input_name(&self, location: SoundInputLocation) -> String {
        format!(
            "{}.{}",
            self.sound_processor(location.processor()).unwrap(),
            self.sound_input(location).unwrap()
        )
    }

    pub(crate) fn combined_argument_name(&self, location: ProcessorArgumentLocation) -> String {
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
        for location in self.expression_results.keys() {
            assert!(graph.contains(location.0));
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

            proc.foreach_expression(|expr, location| {
                for result in expr.graph().results() {
                    assert!(self
                        .expression_results
                        .contains_key(&(location, result.id())));
                }
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
            self.expression_results.iter(),
            |(loc, name), stasher| {
                loc.0.stash(stasher);
                loc.1.stash(stasher);
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
                (
                    ProcessorExpressionLocation::unstash(unstasher)?,
                    ExpressionInputId::unstash(unstasher)?,
                ),
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
            expression_results: expressions,
            sound_inputs,
            sound_processors,
        })
    }
}
