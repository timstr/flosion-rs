use std::collections::{HashMap, HashSet};

use super::{
    expression::SoundExpressionId,
    expressionargument::{
        SoundExpressionArgumentId, SoundExpressionArgumentOrigin, SoundExpressionArgumentOwner,
    },
    path::SoundPath,
    soundgrapherror::SoundError,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
    state::StateOwner,
};

pub(crate) fn find_sound_error(topology: &SoundGraphTopology) -> Option<SoundError> {
    check_missing_ids(topology);

    if let Some(path) = find_sound_cycle(topology) {
        return Some(SoundError::CircularDependency { cycle: path }.into());
    }
    if let Some(err) = validate_sound_connections(topology) {
        return Some(err);
    }
    let bad_dependencies = find_invalid_expression_arguments(topology);
    if bad_dependencies.len() > 0 {
        return Some(SoundError::StateNotInScope { bad_dependencies }.into());
    }
    None
}

pub(super) fn check_missing_ids(topology: &SoundGraphTopology) {
    for sp in topology.sound_processors().values() {
        // for each sound processor
        for i in sp.sound_inputs() {
            // each sound input must exist and list the sound processor as its owner
            match topology.sound_inputs().get(i) {
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
            match topology.expressions().get(i) {
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
            match topology.expression_arguments().get(s) {
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

    for si in topology.sound_inputs().values() {
        // for each sound input
        if let Some(spid) = si.target() {
            if topology.sound_processor(spid).is_none() {
                panic!(
                    "The sound input {:?} lists sound processor {:?} as its target, \
                        but that sound processor does not exist.",
                    si.id(),
                    spid
                )
            }
        }
        match topology.sound_processor(si.owner()) {
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
            match topology.expression_arguments().get(nsid) {
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

    for ns in topology.expression_arguments().values() {
        match ns.owner() {
            // if the argument has an owner, it must exist and list the argument
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                match topology.sound_processor(spid) {
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
            SoundExpressionArgumentOwner::SoundInput(siid) => match topology.sound_input(siid) {
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

    for ni in topology.expressions().values() {
        // for each expression

        // all of its mapped arguments must exist
        for nsid in ni.parameter_mapping().items().values() {
            if topology.expression_arguments().get(nsid).is_none() {
                panic!(
                    "The expression {:?} lists expression argument {:?} as its target, but that \
                        argument does not exist.",
                    ni.id(),
                    nsid
                );
            }
        }

        // its owner must exist and list it as one of its expressions
        match topology.sound_processor(ni.owner()) {
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
            let Some(ns_data) = topology.expression_argument(*nsid) else {
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

pub(super) fn find_sound_cycle(topology: &SoundGraphTopology) -> Option<SoundPath> {
    fn dfs_find_cycle(
        input_id: SoundInputId,
        visited: &mut Vec<SoundInputId>,
        path: &mut SoundPath,
        topo: &SoundGraphTopology,
    ) -> Option<SoundPath> {
        if !visited.contains(&input_id) {
            visited.push(input_id);
        }
        // If the input has already been visited, there is a cycle
        if path.contains_input(input_id) {
            return Some(path.trim_until_input(input_id));
        }
        let input_desc = topo.sound_input(input_id).unwrap();
        let target_id = match input_desc.target() {
            Some(spid) => spid,
            _ => return None,
        };
        let proc_desc = topo.sound_processor(target_id).unwrap();
        path.push(target_id, input_id);
        for target_proc_input in proc_desc.sound_inputs() {
            if let Some(path) = dfs_find_cycle(*target_proc_input, visited, path, topo) {
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
        let input_to_visit = topology
            .sound_inputs()
            .keys()
            .find(|pid| !visited.contains(&pid));
        match input_to_visit {
            None => break None,
            Some(pid) => {
                if let Some(path) = dfs_find_cycle(*pid, &mut visited, &mut path, topology) {
                    break Some(path);
                }
            }
        }
    }
}

pub(super) fn validate_sound_connections(topology: &SoundGraphTopology) -> Option<SoundError> {
    fn visit(
        proc_id: SoundProcessorId,
        states_to_add: usize,
        is_synchronous: bool,
        topo: &SoundGraphTopology,
        init: bool,
    ) -> Option<SoundError> {
        let proc_desc = topo.sound_processor(proc_id).unwrap();
        if proc_desc.instance().is_static() {
            if states_to_add > 1 {
                return Some(SoundError::StaticTooManyStates(proc_id));
            }
            if !is_synchronous {
                return Some(SoundError::StaticNotSynchronous(proc_id));
            }
            if !init {
                return None;
            }
        }
        for input_id in proc_desc.sound_inputs() {
            let input_desc = topo.sound_input(*input_id).unwrap();
            if let Some(t) = input_desc.target() {
                if let Some(err) = visit(
                    t,
                    states_to_add * input_desc.branches().len(),
                    is_synchronous && (input_desc.options() == InputOptions::Synchronous),
                    topo,
                    false,
                ) {
                    return Some(err);
                }
            }
        }
        None
    }
    for proc_desc in topology.sound_processors().values() {
        if proc_desc.instance().is_static() {
            if let Some(err) = visit(proc_desc.id(), 1, true, topology, true) {
                return Some(err);
            }
        }
    }
    None
}

fn state_owner_has_dependency(
    topology: &SoundGraphTopology,
    owner: StateOwner,
    dependency: StateOwner,
) -> bool {
    if owner == dependency {
        return true;
    }
    match owner {
        StateOwner::SoundInput(siid) => {
            let input_desc = topology.sound_input(siid).unwrap();
            if let Some(spid) = input_desc.target() {
                return state_owner_has_dependency(
                    topology,
                    StateOwner::SoundProcessor(spid),
                    dependency,
                );
            }
            return false;
        }
        StateOwner::SoundProcessor(spid) => {
            let proc_desc = topology.sound_processor(spid).unwrap();
            for siid in proc_desc.sound_inputs() {
                if state_owner_has_dependency(topology, StateOwner::SoundInput(*siid), dependency) {
                    return true;
                }
            }
            return false;
        }
    }
}

fn input_depends_on_processor(
    input_id: SoundInputId,
    processor_id: SoundProcessorId,
    topology: &SoundGraphTopology,
) -> bool {
    let input_data = topology.sound_input(input_id).unwrap();
    match input_data.target() {
        Some(spid) => processor_depends_on_processor(spid, processor_id, topology),
        None => false,
    }
}

fn processor_depends_on_processor(
    processor_id: SoundProcessorId,
    other_processor_id: SoundProcessorId,
    topology: &SoundGraphTopology,
) -> bool {
    if processor_id == other_processor_id {
        return true;
    }
    let processor_data = topology.sound_processor(processor_id).unwrap();
    for siid in processor_data.sound_inputs() {
        if input_depends_on_processor(*siid, other_processor_id, topology) {
            return true;
        }
    }
    false
}

pub(super) fn find_invalid_expression_arguments(
    topology: &SoundGraphTopology,
) -> Vec<(SoundExpressionArgumentId, SoundExpressionId)> {
    let mut bad_connections: Vec<(SoundExpressionArgumentId, SoundExpressionId)> = Vec::new();

    for (niid, ni) in topology.expressions() {
        for target in ni.parameter_mapping().items().values() {
            let target_owner = topology.expression_argument(*target).unwrap().owner();
            let depends = match target_owner {
                SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                    processor_depends_on_processor(spid, ni.owner(), topology)
                }
                SoundExpressionArgumentOwner::SoundInput(siid) => {
                    input_depends_on_processor(siid, ni.owner(), topology)
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
    topology: &SoundGraphTopology,
) -> HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>> {
    let mut available_arguments_by_processor: HashMap<
        SoundProcessorId,
        HashSet<SoundExpressionArgumentId>,
    > = HashMap::new();
    for proc_data in topology.sound_processors().values() {
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
            topology
                .sound_processor_targets(processor_id)
                .all(|target_siid| {
                    let parent_sp = topology.sound_input(target_siid).unwrap().owner();
                    cache.contains_key(&parent_sp)
                })
        };

    let sound_input_arguments =
        |input_id: SoundInputId,
         cache: &HashMap<SoundProcessorId, HashSet<SoundExpressionArgumentId>>|
         -> HashSet<SoundExpressionArgumentId> {
            let input_data = topology.sound_input(input_id).unwrap();
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
        let next_proc_id = topology
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
        for target_input in topology.sound_processor_targets(next_proc_id) {
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

        for nsid in topology
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
    for ni_data in topology.expressions().values() {
        let mut available_arguments = available_arguments_by_processor
            .get(&ni_data.owner())
            .unwrap()
            .clone();
        let processor_arguments = topology
            .sound_processor(ni_data.owner())
            .unwrap()
            .expression_arguments();
        for nsid in processor_arguments {
            debug_assert!(available_arguments.contains(nsid));
            let ns_data = topology.expression_argument(*nsid).unwrap();
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
