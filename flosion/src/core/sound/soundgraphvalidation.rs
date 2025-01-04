use std::collections::{hash_map::Entry, HashMap, HashSet};

use super::{
    argument::ProcessorArgumentLocation, expression::ProcessorExpressionLocation,
    sounderror::SoundError, soundgraph::SoundGraph, soundprocessor::SoundProcessorId,
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
    /// How many times will the compiled processor be instantiated?
    num_states: usize,

    /// Is the processor static or isochronically connected to a static
    /// processor, making it basically static?
    logically_static: bool,
}

fn compute_implied_processor_allocations(
    graph: &SoundGraph,
) -> HashMap<SoundProcessorId, ProcessorAllocation> {
    let mut allocations = HashMap::new();

    // Make all static processors static
    for proc in graph.sound_processors().values() {
        if proc.is_static() {
            allocations.insert(
                proc.id(),
                ProcessorAllocation {
                    num_states: 1,
                    logically_static: true,
                },
            );
        }
    }

    // Everything connected isochronically connected to a static
    // processor is also static
    loop {
        let mut anything_changed = false;
        for proc in graph.sound_processors().values() {
            if allocations.contains_key(&proc.id()) {
                continue;
            }

            let mut any_static_isochronic_inputs = false;

            proc.foreach_input(|i, _| {
                if i.category().is_isochronic()
                    && match i.target() {
                        Some(spid) => allocations.contains_key(&spid),
                        None => false,
                    }
                {
                    any_static_isochronic_inputs = true;
                }
            });

            if any_static_isochronic_inputs {
                allocations.insert(
                    proc.id(),
                    ProcessorAllocation {
                        num_states: 1,
                        logically_static: true,
                    },
                );
                anything_changed = true;
            }
        }

        if !anything_changed {
            break;
        }
    }

    // Remaining processors are allocated according to sum of the inputs
    // connected to them
    loop {
        let mut anything_changed = false;

        'searching: for proc in graph.sound_processors().values() {
            let mut sum_of_inbound_allocations: usize = 0;

            if allocations.contains_key(&proc.id()) {
                continue;
            }

            for input_loc in graph.inputs_connected_to(proc.id()) {
                let Some(inbound_proc_allocation) = allocations.get(&input_loc.processor()) else {
                    continue 'searching;
                };
                let inbound_proc_states = inbound_proc_allocation.num_states;
                let branches = graph
                    .sound_processor(input_loc.processor())
                    .unwrap()
                    .with_input(input_loc.input(), |i| i.category().count_branches())
                    .unwrap();

                sum_of_inbound_allocations += inbound_proc_states * branches;
            }

            allocations.insert(
                proc.id(),
                ProcessorAllocation {
                    num_states: sum_of_inbound_allocations,
                    logically_static: false,
                },
            );

            anything_changed = true;
        }

        if !anything_changed {
            break;
        }
    }

    debug_assert!(graph
        .sound_processors()
        .keys()
        .all(|i| allocations.contains_key(i)));
    debug_assert!(allocations
        .keys()
        .all(|i| graph.sound_processor(*i).is_some()));

    allocations
}

pub(super) fn validate_sound_connections(graph: &SoundGraph) -> Option<SoundError> {
    let allocations = compute_implied_processor_allocations(graph);

    println!("*** Allocations ***");
    for (proc_id, allocation) in &allocations {
        println!(
            "    {} : logically_static={} num_states={}",
            graph
                .sound_processor(*proc_id)
                .unwrap()
                .as_graph_object()
                .friendly_name(),
            allocation.logically_static,
            allocation.num_states
        );
    }
    println!("*** *** *** *** ***");

    for (proc_id, allocation) in &allocations {
        let proc_data = graph.sound_processor(*proc_id).unwrap();

        // Static processors must always be static
        assert!(
            !proc_data.is_static() || (allocation.logically_static && allocation.num_states == 1)
        );

        // logically static processors must have all inputs
        // with a state count of 1 and be isochronic
        if allocation.logically_static {
            for input_loc in graph.inputs_connected_to(proc_data.id()) {
                if graph
                    .sound_processor(input_loc.processor())
                    .unwrap()
                    .with_input(input_loc.input(), |i| !i.category().is_isochronic())
                    .unwrap()
                {
                    return Some(SoundError::ConnectionNotIsochronic(input_loc));
                }
                let connected_allocation = allocations.get(&input_loc.processor()).unwrap();
                if !connected_allocation.logically_static || connected_allocation.num_states != 1 {
                    return Some(SoundError::ConnectionNotIsochronic(input_loc));
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
