use std::collections::{HashMap, HashSet};

use hashstash::HashCacheProperty;

use crate::core::{
    sound::{
        argument::ProcessorArgumentLocation, expression::ProcessorExpressionLocation,
        soundgraph::SoundGraph, soundinput::SoundInputLocation, soundprocessor::SoundProcessorId,
    },
    stashing::StashingContext,
};

pub(crate) struct GraphProperties {
    available_inputs: HashCacheProperty<HashMap<SoundProcessorId, HashSet<SoundInputLocation>>>,

    available_arguments:
        HashCacheProperty<HashMap<ProcessorExpressionLocation, HashSet<ProcessorArgumentLocation>>>,
}

impl GraphProperties {
    pub(crate) fn new() -> GraphProperties {
        GraphProperties {
            available_inputs: HashCacheProperty::new(),
            available_arguments: HashCacheProperty::new(),
        }
    }

    pub(crate) fn available_inputs(
        &self,
        processor: SoundProcessorId,
    ) -> Option<&HashSet<SoundInputLocation>> {
        self.available_inputs.get_cached().unwrap().get(&processor)
    }

    pub(crate) fn available_arguments(
        &self,
        location: ProcessorExpressionLocation,
    ) -> Option<&HashSet<ProcessorArgumentLocation>> {
        self.available_arguments
            .get_cached()
            .unwrap()
            .get(&location)
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        self.available_inputs.refresh1_with_context(
            available_sound_inputs,
            graph,
            StashingContext::new_checking_recompilation(),
        );

        self.available_arguments.refresh1_with_context(
            |graph| {
                available_sound_expression_arguments(
                    graph,
                    &self.available_inputs.get_cached().unwrap(),
                )
            },
            graph,
            StashingContext::new_checking_recompilation(),
        );
    }
}

/// Returns a hashmap containing for each sound processor, the full set
/// of sound inputs that are always up the audio stack when it is invoked
pub(super) fn available_sound_inputs(
    graph: &SoundGraph,
) -> HashMap<SoundProcessorId, HashSet<SoundInputLocation>> {
    let mut cached_inputs = HashMap::new();

    // All static processors have no available inputs
    for proc in graph.sound_processors().values() {
        if proc.is_static() {
            cached_inputs.insert(proc.id(), HashSet::new());
        }
    }

    loop {
        // Find the next processor whose targets are all cached
        let Some(next_proc) = graph.sound_processors().values().find(|proc| {
            // not yet cached
            if cached_inputs.contains_key(&proc.id()) {
                return false;
            }
            // all targets are cached
            if !graph
                .inputs_connected_to(proc.id())
                .into_iter()
                .all(|target| cached_inputs.contains_key(&target.processor()))
            {
                return false;
            }
            true
        }) else {
            break;
        };

        // The set of available inputs is the intersection of inputs available
        // through each target
        let targets = graph.inputs_connected_to(next_proc.id());

        let available_inputs;

        if let Some((head, rest)) = targets.split_first() {
            let mut inputs = cached_inputs.get(&head.processor()).unwrap().clone();

            inputs.insert(*head);

            for target in rest {
                let target_cached_inputs = cached_inputs.get(&target.processor()).unwrap();
                inputs.retain(|i| target_cached_inputs.contains(i));
            }

            available_inputs = inputs;
        } else {
            available_inputs = HashSet::new();
        }

        cached_inputs.insert(next_proc.id(), available_inputs);
    }

    cached_inputs
}

pub(crate) fn available_sound_expression_arguments(
    graph: &SoundGraph,
    available_sound_inputs: &HashMap<SoundProcessorId, HashSet<SoundInputLocation>>,
) -> HashMap<ProcessorExpressionLocation, HashSet<ProcessorArgumentLocation>> {
    let mut available_arguments = HashMap::new();

    // The set of arguments available to each expression is those arguments
    // available to its processor via sound inputs, plus any that are
    // additionally in scope
    for proc in graph.sound_processors().values() {
        proc.foreach_expression(|expr, loc| {
            let mut args = HashSet::new();

            // Add arguments from all available inputs
            for input in available_sound_inputs.get(&loc.processor()).unwrap() {
                graph
                    .sound_processor(input.processor())
                    .unwrap()
                    .with_input(input.input(), |i| {
                        for a in i.argument_scope().arguments() {
                            args.insert(ProcessorArgumentLocation::new(input.processor(), *a));
                        }
                    })
                    .unwrap();
            }

            // Add arguments from the expression's own scope
            for a in expr.scope().arguments() {
                args.insert(ProcessorArgumentLocation::new(proc.id(), *a));
            }

            available_arguments.insert(loc, args);
        });
    }

    available_arguments
}
