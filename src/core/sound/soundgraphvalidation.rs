use std::collections::{hash_map::Entry, HashMap, HashSet};

use crate::core::sound::expressionargument::ProcessorArgumentDataSource;

use super::{
    expression::{ProcessorExpression, ProcessorExpressionLocation},
    expressionargument::{ArgumentLocation, ProcessorArgumentLocation, SoundInputArgumentLocation},
    sounderror::SoundError,
    soundgraph::SoundGraph,
    soundinput::{InputOptions, SoundInputLocation},
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
    fn dfs_find_cycle(
        processor_id: SoundProcessorId,
        visited_inputs: &mut HashSet<SoundInputLocation>,
        graph: &SoundGraph,
    ) -> bool {
        let mut any_cycles = false;
        graph
            .sound_processor(processor_id)
            .unwrap()
            .foreach_input(|input, location| {
                if visited_inputs.contains(&location) {
                    any_cycles = true;
                    return;
                }
                visited_inputs.insert(location);
                if let Some(target_id) = input.target() {
                    dfs_find_cycle(target_id, visited_inputs, graph);
                }
            });
        any_cycles
    }

    let mut visited_processors: HashSet<SoundProcessorId> = HashSet::new();

    loop {
        let Some(proc_to_visit) = graph
            .sound_processors()
            .keys()
            .find(|pid| !visited_processors.contains(&pid))
            .cloned()
        else {
            return false;
        };

        let mut visited_inputs = HashSet::new();
        if dfs_find_cycle(proc_to_visit, &mut visited_inputs, graph) {
            return true;
        }
        for input_location in visited_inputs {
            visited_processors.insert(input_location.processor());
        }
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
        let is_static = proc_data.instance().is_static();

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

            let states = processor_states * input.branches().len();

            let input_is_sync = match input.options() {
                InputOptions::Synchronous => processor_is_sync,
                InputOptions::NonSynchronous => false,
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

        if proc_data.instance().is_static() {
            // Static processors must always be sync
            if !allocation.always_sync {
                return Some(SoundError::StaticNotSynchronous(*proc_id));
            }

            // Static processors must be allocated one state per input
            // We don't check the processor's own implied number of states
            // because that would overcount if there are multiple inputs.
            for input_id in graph.sound_processor_targets(*proc_id) {
                let num_input_branches = graph
                    .with_sound_input(input_id, |input| input.branches().len())
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

fn input_depends_on_processor(
    input_location: SoundInputLocation,
    processor_id: SoundProcessorId,
    graph: &SoundGraph,
) -> bool {
    let input_target = graph
        .with_sound_input(input_location, |input| input.target())
        .unwrap();
    match input_target {
        Some(spid) => processor_depends_on_processor(spid, processor_id, graph),
        None => false,
    }
}

fn processor_depends_on_processor(
    processor_id: SoundProcessorId,
    other_processor_id: SoundProcessorId,
    graph: &SoundGraph,
) -> bool {
    if processor_id == other_processor_id {
        return true;
    }
    let mut any_inputs_depend = false;
    graph
        .sound_processor(processor_id)
        .unwrap()
        .foreach_input(|_, location| {
            if input_depends_on_processor(location, other_processor_id, graph) {
                any_inputs_depend = true;
            }
        });
    false
}

pub(super) fn find_invalid_expression_arguments(
    graph: &SoundGraph,
) -> Vec<(ArgumentLocation, ProcessorExpressionLocation)> {
    let mut bad_connections: Vec<(ArgumentLocation, ProcessorExpressionLocation)> = Vec::new();

    for proc_data in graph.sound_processors().values() {
        proc_data.foreach_expression(|expr| {
            for target in expr.mapping().items().values() {
                let depends = match target {
                    ArgumentLocation::Processor(arg_location) => processor_depends_on_processor(
                        arg_location.processor(),
                        proc_data.id(),
                        graph,
                    ),
                    ArgumentLocation::Input(arg_location) => {
                        let input_location =
                            SoundInputLocation::new(arg_location.processor(), arg_location.input());
                        input_depends_on_processor(input_location, proc_data.id(), graph)
                    }
                };
                if !depends {
                    bad_connections.push((
                        *target,
                        ProcessorExpressionLocation::new(proc_data.id(), expr.id()),
                    ));
                }
            }
        });
    }

    return bad_connections;
}

// TODO: move to soundgraphproperties?
pub(crate) fn available_sound_expression_arguments(
    graph: &SoundGraph,
) -> HashMap<ProcessorExpressionLocation, HashSet<ArgumentLocation>> {
    let mut available_arguments_by_processor: HashMap<SoundProcessorId, HashSet<ArgumentLocation>> =
        HashMap::new();
    for proc_data in graph.sound_processors().values() {
        if proc_data.instance().is_static() {
            let mut static_args = HashSet::<ArgumentLocation>::new();
            proc_data.foreach_processor_argument(|arg| {
                let location = ProcessorArgumentLocation::new(proc_data.id(), arg.id());
                static_args.insert(ArgumentLocation::Processor(location));
            });
            available_arguments_by_processor.insert(proc_data.id(), static_args);
        }
    }

    let all_targets_cached_for =
        |processor_id: SoundProcessorId,
         cache: &HashMap<SoundProcessorId, HashSet<ArgumentLocation>>| {
            graph
                .sound_processor_targets(processor_id)
                .iter()
                .all(|target_siid| cache.contains_key(&target_siid.processor()))
        };

    let sound_input_arguments = |input_location: SoundInputLocation,
                                 cache: &HashMap<SoundProcessorId, HashSet<ArgumentLocation>>|
     -> HashSet<ArgumentLocation> {
        let mut arguments = cache
            .get(&input_location.processor())
            .expect("Processor expression arguments should have been cached")
            .clone();
        graph
            .sound_processor(input_location.processor())
            .unwrap()
            .foreach_input_argument(|arg| {
                arguments.insert(ArgumentLocation::Input(SoundInputArgumentLocation::new(
                    input_location.processor(),
                    input_location.input(),
                    arg.id(),
                )));
            });
        arguments
    };

    // Cache all processors in topological order
    loop {
        let next_proc_id = graph
            .sound_processors()
            .values()
            .filter_map(|proc_data| {
                // don't revisit processors that are already cached
                if available_arguments_by_processor.contains_key(&proc_data.id()) {
                    return None;
                }
                // visit processors for which all targets are cached
                if all_targets_cached_for(proc_data.id(), &available_arguments_by_processor) {
                    Some(proc_data.id())
                } else {
                    None
                }
            })
            .next();

        let Some(next_proc_id) = next_proc_id else {
            // All done!
            break;
        };

        let mut available_arguments: Option<HashSet<ArgumentLocation>> = None;

        // Available upstream arguments are the intersection of all those
        // available via each destination sound input
        for target_input in graph.sound_processor_targets(next_proc_id) {
            let target_input_arguments =
                sound_input_arguments(target_input, &available_arguments_by_processor);
            if let Some(arguments) = available_arguments.as_mut() {
                *arguments = arguments
                    .intersection(&target_input_arguments)
                    .cloned()
                    .collect();
            } else {
                available_arguments = Some(target_input_arguments);
            }
        }

        let mut available_arguments = available_arguments.unwrap_or_else(HashSet::new);

        graph
            .sound_processor(next_proc_id)
            .unwrap()
            .foreach_processor_argument(|arg| {
                available_arguments.insert(ArgumentLocation::Processor(
                    ProcessorArgumentLocation::new(next_proc_id, arg.id()),
                ));
            });

        available_arguments_by_processor.insert(next_proc_id, available_arguments);
    }

    let mut available_arguments_by_expression = HashMap::new();

    // Each expression's available arguments are those available from the processor minus
    // any out-of-scope locals
    for proc_data in graph.sound_processors().values() {
        proc_data.foreach_expression(|expr: &ProcessorExpression| {
            let mut available_arguments = available_arguments_by_processor
                .get(&proc_data.id())
                .unwrap()
                .clone();

            proc_data.foreach_processor_argument(|arg| {
                let location = ArgumentLocation::Processor(ProcessorArgumentLocation::new(
                    proc_data.id(),
                    arg.id(),
                ));
                debug_assert!(available_arguments.contains(&location));
                match arg.instance().data_source() {
                    ProcessorArgumentDataSource::ProcessorState => {
                        if !expr.scope().processor_state_available() {
                            available_arguments.remove(&location);
                        }
                    }
                    ProcessorArgumentDataSource::LocalVariable => {
                        if !expr.scope().available_local_arguments().contains(&arg.id()) {
                            available_arguments.remove(&location);
                        }
                    }
                }
            });

            let location = ProcessorExpressionLocation::new(proc_data.id(), expr.id());

            available_arguments_by_expression.insert(location, available_arguments);
        });
    }

    available_arguments_by_expression
}
