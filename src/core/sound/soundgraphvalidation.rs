use std::collections::{hash_map::Entry, HashMap, HashSet};

use super::{
    expression::SoundExpressionId,
    expressionargument::{
        SoundExpressionArgumentId, SoundExpressionArgumentOrigin, SoundExpressionArgumentOwner,
    },
    path::SoundPath,
    sounderror::SoundError,
    soundgraph::SoundGraph,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
};

pub(super) fn find_sound_error(graph: &SoundGraph) -> Option<SoundError> {
    check_missing_ids(graph);

    if let Some(path) = find_sound_cycle(graph) {
        return Some(SoundError::CircularDependency { cycle: path });
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

pub(super) fn check_missing_ids(graph: &SoundGraph) {
    for sp in graph.sound_processors().values() {
        // for each sound processor
        for i in sp.sound_inputs() {
            // each sound input must exist and list the sound processor as its owner
            match graph.sound_inputs().get(i) {
                Some(idata) => {
                    if idata.owner() != sp.id() {
                        panic!(
                            "Sound processor {:?} has sound input {:?} listed as an \
                                input, but that input does not list the sound processor \
                                as its owner.",
                            sp.id(),
                            i
                        );
                    }
                }
                None => panic!(
                    "Sound processor {:?} has sound input {:?} listed as an input, \
                        but that input does not exist.",
                    sp.id(),
                    i
                ),
            }
        }
        for i in sp.expressions() {
            // each expression must exist and list the sound processor as its owner
            match graph.expressions().get(i) {
                Some(idata) => {
                    if idata.owner() != sp.id() {
                        panic!(
                            "Sound processor {:?} lists expression {:?} as one \
                            of its expressions, but that expression does not list
                            the sound processor as its owner.",
                            sp.id(),
                            i
                        );
                    }
                }
                None => panic!(
                    "Sound processor {:?} lists expression {:?} as one of its \
                        expressions, but that expression does not exist.",
                    sp.id(),
                    i
                ),
            }
        }
        for s in sp.expression_arguments() {
            // each argument must exist and list the sound processor as its owner
            match graph.expression_arguments().get(s) {
                Some(sdata) => {
                    if sdata.owner() != SoundExpressionArgumentOwner::SoundProcessor(sp.id()) {
                        panic!(
                            "Sound processor {:?} lists expression argument {:?} as one \
                                of its arguments, but that argument doesn't \
                                list the sound processor as its owner.",
                            sp.id(),
                            s
                        );
                    }
                }
                None => panic!(
                    "Sound processor {:?} lists expression argument {:?} as one of its \
                        arguments, but that argument does not exist.",
                    sp.id(),
                    s
                ),
            }
        }
    }

    for si in graph.sound_inputs().values() {
        // for each sound input
        if let Some(spid) = si.target() {
            if graph.sound_processor(spid).is_none() {
                panic!(
                    "The sound input {:?} lists sound processor {:?} as its target, \
                        but that sound processor does not exist.",
                    si.id(),
                    spid
                )
            }
        }
        match graph.sound_processor(si.owner()) {
            // its owner must exist and list the input
            Some(sp) => {
                if !sp.sound_inputs().contains(&si.id()) {
                    panic!(
                        "Sound input {:?} lists sound processor {:?} as its owner, \
                            but that sound processor doesn't list the sound input as one \
                            of its inputs.",
                        si.id(),
                        si.owner()
                    );
                }
            }
            None => panic!(
                "Sound input {:?} lists sound processor {:?} as its owner, but that \
                    sound processor does not exist.",
                si.id(),
                si.owner()
            ),
        }
        for nsid in si.expression_arguments() {
            // each argument must exist and list the sound input as its owner
            match graph.expression_arguments().get(nsid) {
                Some(ns) => {
                    if ns.owner() != SoundExpressionArgumentOwner::SoundInput(si.id()) {
                        panic!(
                            "Sound input {:?} lists expression argument {:?} as one of its \
                                arguments, but that argument doesn't list the \
                                sound input as its owner.",
                            si.id(),
                            nsid
                        );
                    }
                }
                None => panic!(
                    "Sound input {:?} lists expression argument {:?} as one of its arguments, \
                        but that argument does not exist.",
                    si.id(),
                    nsid
                ),
            }
        }
    }

    for ns in graph.expression_arguments().values() {
        match ns.owner() {
            // if the argument has an owner, it must exist and list the argument
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                match graph.sound_processor(spid) {
                    Some(sp) => {
                        if !sp.expression_arguments().contains(&ns.id()) {
                            panic!(
                                "The expression argument {:?} lists sound processor {:?} as its owner, \
                                but that sound processor does not list the argument as one \
                                of its arguments.",
                                ns.id(),
                                spid
                            );
                        }
                    }
                    None => panic!(
                        "The expression argument {:?} lists sound processor {:?} as its owner, but that \
                        sound processor does not exist.",
                        ns.id(),
                        spid
                    ),
                }
            }
            SoundExpressionArgumentOwner::SoundInput(siid) => match graph.sound_input(siid) {
                Some(si) => {
                    if !si.expression_arguments().contains(&ns.id()) {
                        panic!(
                            "The expression argument {:?} lists sound input {:?} as its owner, \
                                but that sound input does not list the argument as one \
                                of its arguments.",
                            ns.id(),
                            siid
                        );
                    }
                }
                None => panic!(
                    "The expression argument {:?} lists sound input {:?} as its owner, but that \
                        sound input doesn't exist.",
                    ns.id(),
                    siid
                ),
            },
        }
    }

    for ni in graph.expressions().values() {
        // for each expression

        // all of its mapped arguments must exist
        for nsid in ni.parameter_mapping().items().values() {
            if graph.expression_arguments().get(nsid).is_none() {
                panic!(
                    "The expression {:?} lists expression argument {:?} as its target, but that \
                        argument does not exist.",
                    ni.id(),
                    nsid
                );
            }
        }

        // its owner must exist and list it as one of its expressions
        match graph.sound_processor(ni.owner()) {
            Some(sp) => {
                if !sp.expressions().contains(&ni.id()) {
                    panic!(
                        "The expression {:?} lists sound processor {:?} as its owner, \
                                but that sound processor doesn't list the expression as one of \
                                its expressions.",
                        ni.id(),
                        ni.owner()
                    );
                }
            }
            None => panic!(
                "The expression {:?} lists sound processor {:?} as its owner, but that \
                        sound processor does not exist.",
                ni.id(),
                ni.owner()
            ),
        }

        // any expression arguments listed in its scope must belong to the parent
        // sound processor and be local arguments
        for nsid in ni.scope().available_local_arguments() {
            let Some(ns_data) = graph.expression_argument(*nsid) else {
                panic!(
                    "The expression {:?} lists expression argument {:?} as in its local scope, but \
                    that argument doesn't exist.",
                    ni.id(),
                    nsid
                );
            };
            if ns_data.owner() != SoundExpressionArgumentOwner::SoundProcessor(ni.owner()) {
                panic!(
                    "The expression {:?} lists expression argument {:?} as in its local scope, but \
                    that argument doesn't belong to the same sound processor.",
                    ni.id(),
                    nsid
                );
            }
            if ns_data.instance().origin() != SoundExpressionArgumentOrigin::Local(ni.owner()) {
                panic!(
                    "The expression {:?} lists expression argument {:?} as in its local scope, but \
                    that argument is not a local argument.",
                    ni.id(),
                    nsid
                );
            }
        }
    }

    // whew, made it
}

pub(super) fn find_sound_cycle(graph: &SoundGraph) -> Option<SoundPath> {
    fn dfs_find_cycle(
        input_id: SoundInputId,
        visited: &mut Vec<SoundInputId>,
        path: &mut SoundPath,
        graph: &SoundGraph,
    ) -> Option<SoundPath> {
        if !visited.contains(&input_id) {
            visited.push(input_id);
        }
        // If the input has already been visited, there is a cycle
        if path.contains_input(input_id) {
            return Some(path.trim_until_input(input_id));
        }
        let input_desc = graph.sound_input(input_id).unwrap();
        let target_id = match input_desc.target() {
            Some(spid) => spid,
            _ => return None,
        };
        let proc_desc = graph.sound_processor(target_id).unwrap();
        path.push(target_id, input_id);
        for target_proc_input in proc_desc.sound_inputs() {
            if let Some(path) = dfs_find_cycle(*target_proc_input, visited, path, graph) {
                return Some(path);
            }
        }
        path.pop();
        None
    }

    let mut visited: Vec<SoundInputId> = Vec::new();
    let mut path: SoundPath = SoundPath::new(Vec::new());

    loop {
        assert_eq!(path.connections.len(), 0);
        let input_to_visit = graph
            .sound_inputs()
            .keys()
            .find(|pid| !visited.contains(&pid));
        match input_to_visit {
            None => break None,
            Some(pid) => {
                if let Some(path) = dfs_find_cycle(*pid, &mut visited, &mut path, graph) {
                    break Some(path);
                }
            }
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

        for input_id in proc_data.sound_inputs() {
            let input_data = graph.sound_input(*input_id).unwrap();
            let Some(target_proc_id) = input_data.target() else {
                continue;
            };

            let states = processor_states * input_data.branches().len();

            let input_is_sync = match input_data.options() {
                InputOptions::Synchronous => processor_is_sync,
                InputOptions::NonSynchronous => false,
            };
            let sync = is_sync && input_is_sync;

            visit(target_proc_id, states, sync, graph, allocations);
        }
    }

    // find all processors with no dependents
    let roots: Vec<SoundProcessorId>;
    {
        let mut processors_with_dependents = HashSet::<SoundProcessorId>::new();
        for proc in graph.sound_processors().values() {
            for input_id in proc.sound_inputs() {
                let input = graph.sound_input(*input_id).unwrap();
                if let Some(target) = input.target() {
                    processors_with_dependents.insert(target);
                }
            }
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
                let input = graph.sound_input(input_id).unwrap();
                if input.branches().len() != 1
                    || allocations.get(&input.owner()).unwrap().implied_num_states != 1
                {
                    return Some(SoundError::StaticNotOneState(*proc_id));
                }
            }
        }
    }

    None
}

fn input_depends_on_processor(
    input_id: SoundInputId,
    processor_id: SoundProcessorId,
    graph: &SoundGraph,
) -> bool {
    let input_data = graph.sound_input(input_id).unwrap();
    match input_data.target() {
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
    let processor_data = graph.sound_processor(processor_id).unwrap();
    for siid in processor_data.sound_inputs() {
        if input_depends_on_processor(*siid, other_processor_id, graph) {
            return true;
        }
    }
    false
}

pub(super) fn find_invalid_expression_arguments(
    graph: &SoundGraph,
) -> Vec<(SoundExpressionArgumentId, SoundExpressionId)> {
    let mut bad_connections: Vec<(SoundExpressionArgumentId, SoundExpressionId)> = Vec::new();

    for (niid, ni) in graph.expressions() {
        for target in ni.parameter_mapping().items().values() {
            let target_owner = graph.expression_argument(*target).unwrap().owner();
            let depends = match target_owner {
                SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                    processor_depends_on_processor(spid, ni.owner(), graph)
                }
                SoundExpressionArgumentOwner::SoundInput(siid) => {
                    input_depends_on_processor(siid, ni.owner(), graph)
                }
            };
            if !depends {
                bad_connections.push((*target, *niid));
            }
        }
    }

    return bad_connections;
}

pub(crate) fn available_sound_expression_arguments(
    graph: &SoundGraph,
) -> HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>> {
    let mut available_arguments_by_processor: HashMap<
        SoundProcessorId,
        HashSet<SoundExpressionArgumentId>,
    > = HashMap::new();
    for proc_data in graph.sound_processors().values() {
        if proc_data.instance().is_static() {
            available_arguments_by_processor.insert(
                proc_data.id(),
                proc_data.expression_arguments().iter().cloned().collect(),
            );
        }
    }

    let all_targets_cached_for =
        |processor_id: SoundProcessorId,
         cache: &HashMap<SoundProcessorId, HashSet<SoundExpressionArgumentId>>| {
            graph
                .sound_processor_targets(processor_id)
                .all(|target_siid| {
                    let parent_sp = graph.sound_input(target_siid).unwrap().owner();
                    cache.contains_key(&parent_sp)
                })
        };

    let sound_input_arguments =
        |input_id: SoundInputId,
         cache: &HashMap<SoundProcessorId, HashSet<SoundExpressionArgumentId>>|
         -> HashSet<SoundExpressionArgumentId> {
            let input_data = graph.sound_input(input_id).unwrap();
            let mut arguments = cache
                .get(&input_data.owner())
                .expect("Processor expression arguments should have been cached")
                .clone();
            for nsid in input_data.expression_arguments() {
                arguments.insert(*nsid);
            }
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

        let mut available_arguments: Option<HashSet<SoundExpressionArgumentId>> = None;

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

        for nsid in graph
            .sound_processor(next_proc_id)
            .unwrap()
            .expression_arguments()
        {
            available_arguments.insert(*nsid);
        }

        available_arguments_by_processor.insert(next_proc_id, available_arguments);
    }

    let mut available_arguments_by_expression = HashMap::new();

    // Each expression's available arguments are those available from the processor minus
    // any out-of-scope locals
    for ni_data in graph.expressions().values() {
        let mut available_arguments = available_arguments_by_processor
            .get(&ni_data.owner())
            .unwrap()
            .clone();
        let processor_arguments = graph
            .sound_processor(ni_data.owner())
            .unwrap()
            .expression_arguments();
        for nsid in processor_arguments {
            debug_assert!(available_arguments.contains(nsid));
            let ns_data = graph.expression_argument(*nsid).unwrap();
            match ns_data.instance().origin() {
                SoundExpressionArgumentOrigin::ProcessorState(spid) => {
                    debug_assert_eq!(spid, ni_data.owner());
                    if !ni_data.scope().processor_state_available() {
                        available_arguments.remove(nsid);
                    }
                }
                SoundExpressionArgumentOrigin::InputState(_) => {
                    panic!("Processor expression argument can't have a sound input as its origin");
                }
                SoundExpressionArgumentOrigin::Local(spid) => {
                    debug_assert_eq!(spid, ni_data.owner());
                    if !ni_data.scope().available_local_arguments().contains(nsid) {
                        available_arguments.remove(nsid);
                    }
                }
            }
        }
        available_arguments_by_expression.insert(ni_data.id(), available_arguments);
    }

    available_arguments_by_expression
}
