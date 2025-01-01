use std::collections::{hash_map::Entry, HashMap, HashSet};

use crate::core::sound::soundinput::SoundInputBranching;

use super::{
    argument::ProcessorArgumentLocation, expression::ProcessorExpressionLocation,
    sounderror::SoundError, soundgraph::SoundGraph, soundinput::Chronicity,
    soundprocessor::SoundProcessorId,
};

pub(super) fn find_sound_error(graph: &SoundGraph) -> Option<SoundError> {
    if find_sound_cycle(graph) {
        return Some(SoundError::CircularDependency);
    }
    if let Some(err) = validate_sound_connections(graph) {
        return Some(err);
    }
    let bad_dependencies = find_invalid_expression_arguments(graph);
    if bad_dependencies.len() > 0 {
        return Some(SoundError::StateNotInScope { bad_dependencies });
    }
    None
}

pub(super) fn find_sound_cycle(graph: &SoundGraph) -> bool {
    fn find_cycle(
        proc_id: SoundProcessorId,
        current_path: &mut Vec<SoundProcessorId>,
        all_visited_procs: &mut HashSet<SoundProcessorId>,
        found_a_cyle: &mut bool,
        graph: &SoundGraph,
    ) {
        if current_path.contains(&proc_id) {
            *found_a_cyle = true;
            return;
        }
        if all_visited_procs.contains(&proc_id) {
            return;
        }
        all_visited_procs.insert(proc_id);
        graph
            .sound_processor(proc_id)
            .unwrap()
            .foreach_input(|input, _| {
                if let Some(target_id) = input.target() {
                    current_path.push(proc_id);
                    find_cycle(
                        target_id,
                        current_path,
                        all_visited_procs,
                        found_a_cyle,
                        graph,
                    );
                    current_path.pop();
                }
            });
    }

    let mut visited_procs: HashSet<SoundProcessorId> = HashSet::new();

    loop {
        let Some(proc_to_visit) = graph
            .sound_processors()
            .keys()
            .find(|pid| !visited_procs.contains(&pid))
            .cloned()
        else {
            return false;
        };
        let mut path = Vec::new();
        let mut found_a_cycle = false;
        find_cycle(
            proc_to_visit,
            &mut path,
            &mut visited_procs,
            &mut found_a_cycle,
            graph,
        );
        if found_a_cycle {
            return true;
        }
        visited_procs.insert(proc_to_visit);
    }
}

struct ProcessorAllocation {
    implied_num_states: usize,
    always_sync: bool,
}

fn compute_implied_processor_allocations(
    graph: &SoundGraph,
) -> HashMap<SoundProcessorId, ProcessorAllocation> {
    fn visit(
        processor_id: SoundProcessorId,
        states_to_add: usize,
        is_sync: bool,
        graph: &SoundGraph,
        allocations: &mut HashMap<SoundProcessorId, ProcessorAllocation>,
    ) {
        let proc_data = graph.sound_processor(processor_id).unwrap();
        let is_static = proc_data.is_static();

        match allocations.entry(processor_id) {
            Entry::Occupied(mut entry) => {
                // The processor has been visited already.

                let proc_sum = entry.get_mut();

                proc_sum.always_sync &= is_sync;

                if is_static {
                    // If it is static, it always implies a single
                    // state being added via its inputs, so it
                    // only needs to be visited once.
                    // return;
                } else {
                    proc_sum.implied_num_states += states_to_add;
                }
            }
            Entry::Vacant(entry) => {
                // The processor is being visited for the first time.
                entry.insert(ProcessorAllocation {
                    implied_num_states: states_to_add,
                    always_sync: is_sync,
                });
            }
        }

        let processor_is_sync = is_sync || is_static;
        let processor_states = if is_static { 1 } else { states_to_add };

        proc_data.foreach_input(|input, _| {
            let Some(target_proc_id) = input.target() else {
                return;
            };

            // TODO: disallow branching inputs with 1 branch to be connected
            // to a static processor
            let states = processor_states * input.branching().count();

            let input_is_sync = match input.chronicity() {
                Chronicity::Iso => processor_is_sync,
                Chronicity::Aniso => false,
            };
            let sync = is_sync && input_is_sync;

            visit(target_proc_id, states, sync, graph, allocations);
        });
    }

    // find all processors with no dependents
    let roots: Vec<SoundProcessorId>;
    {
        let mut processors_with_dependents = HashSet::<SoundProcessorId>::new();
        for proc in graph.sound_processors().values() {
            proc.foreach_input(|input, _| {
                if let Some(target) = input.target() {
                    processors_with_dependents.insert(target);
                }
            });
        }

        let mut processors_without_dependents = Vec::<SoundProcessorId>::new();

        for proc_id in graph.sound_processors().keys() {
            if !processors_with_dependents.contains(proc_id) {
                processors_without_dependents.push(*proc_id);
            }
        }

        roots = processors_without_dependents;
    }

    let mut allocations = HashMap::new();

    // Visit all root processors and populate them and their dependencies
    for spid in roots {
        visit(spid, 1, true, graph, &mut allocations);
    }

    allocations
}

pub(super) fn validate_sound_connections(graph: &SoundGraph) -> Option<SoundError> {
    let allocations = compute_implied_processor_allocations(graph);

    for (proc_id, allocation) in &allocations {
        let proc_data = graph.sound_processor(*proc_id).unwrap();

        if proc_data.is_static() {
            // Static processors must always be sync
            if !allocation.always_sync {
                return Some(SoundError::StaticNotSynchronous(*proc_id));
            }

            // Static processors must be allocated one state per input
            // We don't check the processor's own implied number of states
            // because that would overcount if there are multiple inputs.
            for input_id in graph.inputs_connected_to(*proc_id) {
                let num_input_branches = graph
                    .with_sound_input(input_id, |input| {
                        // TODO: disallow Branched(1) to be connected to static
                        // processors
                        input.branching().count()
                    })
                    .unwrap();
                let num_implied_states = allocations
                    .get(&input_id.processor())
                    .unwrap()
                    .implied_num_states;
                if num_input_branches != 1 || num_implied_states != 1 {
                    return Some(SoundError::StaticNotOneState(*proc_id));
                }
            }
        }
    }

    None
}

pub(super) fn find_invalid_expression_arguments(
    graph: &SoundGraph,
) -> Vec<(ProcessorArgumentLocation, ProcessorExpressionLocation)> {
    // HACK: everything is valid
    Vec::new()
}
