use std::collections::{HashMap, HashSet};

use super::{
    path::SoundPath,
    soundedit::SoundEdit,
    soundgrapherror::SoundError,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::{SoundNumberSourceId, SoundNumberSourceOwner},
    soundprocessor::SoundProcessorId,
    state::StateOwner,
};

pub(crate) fn find_error(topology: &SoundGraphTopology) -> Option<SoundError> {
    check_missing_ids(topology);

    if let Some(path) = find_sound_cycle(topology) {
        return Some(SoundError::CircularDependency { cycle: path }.into());
    }
    if let Some(err) = validate_sound_connections(topology) {
        return Some(err);
    }
    let bad_dependencies = find_invalid_number_connections(topology);
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
        for i in sp.number_inputs() {
            // each number input must exist and list the sound processor as its owner
            match topology.number_inputs().get(i) {
                Some(idata) => {
                    if idata.owner() != sp.id() {
                        panic!(
                            "Sound processor {:?} lists number input {:?} as one \
                            of its number inputs, but that number input does not list
                            the sound processor as its owner.",
                            sp.id(),
                            i
                        );
                    }
                }
                None => panic!(
                    "Sound processor {:?} lists number input {:?} as one of its \
                        number inputs, but that number input does not exist.",
                    sp.id(),
                    i
                ),
            }
        }
        for s in sp.number_sources() {
            // each number source must exist and list the sound processor as its owner
            match topology.number_sources().get(s) {
                Some(sdata) => {
                    if sdata.owner() != SoundNumberSourceOwner::SoundProcessor(sp.id()) {
                        panic!(
                            "Sound processor {:?} lists number source {:?} as one \
                                of its number sources, but that number source doesn't \
                                list the sound processor as its owner.",
                            sp.id(),
                            s
                        );
                    }
                }
                None => panic!(
                    "Sound processor {:?} lists number source {:?} as one of its \
                        number sources, but that number source does not exist.",
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
        for nsid in si.number_sources() {
            // each number source must exist and list the sound input as its owner
            match topology.number_sources().get(nsid) {
                Some(ns) => {
                    if ns.owner() != SoundNumberSourceOwner::SoundInput(si.id()) {
                        panic!(
                            "Sound input {:?} lists number source {:?} as one of its \
                                number sources, but that number source doesn't list the \
                                sound input as its owner.",
                            si.id(),
                            nsid
                        );
                    }
                }
                None => panic!(
                    "Sound input {:?} lists number source {:?} as one of its number \
                        sources, but that number source does not exist.",
                    si.id(),
                    nsid
                ),
            }
        }
    }

    for ns in topology.number_sources().values() {
        match ns.owner() {
            // if the number source has an owner, it must exist and list the number source
            SoundNumberSourceOwner::SoundProcessor(spid) => match topology.sound_processor(spid) {
                Some(sp) => {
                    if !sp.number_sources().contains(&ns.id()) {
                        panic!(
                            "The number source {:?} lists sound processor {:?} as its owner, \
                                but that sound processor does not list the number source as one \
                                of its number sources.",
                            ns.id(),
                            spid
                        );
                    }
                }
                None => panic!(
                    "The number source {:?} lists sound processor {:?} as its owner, but that \
                        sound processor does not exist.",
                    ns.id(),
                    spid
                ),
            },
            SoundNumberSourceOwner::SoundInput(siid) => match topology.sound_input(siid) {
                Some(si) => {
                    if !si.number_sources().contains(&ns.id()) {
                        panic!(
                            "The number source {:?} lists sound input {:?} as its owner, \
                                but that sound input does not list the number source as one \
                                of its number sources.",
                            ns.id(),
                            siid
                        );
                    }
                }
                None => panic!(
                    "The number source {:?} lists sound input {:?} as its owner, but that \
                        sound input doesn't exist.",
                    ns.id(),
                    siid
                ),
            },
        }
    }

    for ni in topology.number_inputs().values() {
        // for all number inputs
        for nsid in ni.target_mapping().items().values() {
            // its targets must exist
            if topology.number_sources().get(nsid).is_none() {
                panic!(
                    "The number input {:?} lists number source {:?} as its target, but that \
                        number source does not exist.",
                    ni.id(),
                    nsid
                );
            }
        }
        match topology.sound_processor(ni.owner()) {
            Some(sp) => {
                if !sp.number_inputs().contains(&ni.id()) {
                    panic!(
                        "The number input {:?} lists sound processor {:?} as its owner, \
                                but that sound processor doesn't list the number input as one of \
                                its number inputs.",
                        ni.id(),
                        ni.owner()
                    );
                }
            }
            None => panic!(
                "The number input {:?} lists sound processor {:?} as its owner, but that \
                        sound processor does not exist.",
                ni.id(),
                ni.owner()
            ),
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
                    states_to_add * input_desc.num_keys(),
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

pub(super) fn find_invalid_number_connections(
    topology: &SoundGraphTopology,
) -> Vec<(SoundNumberSourceId, SoundNumberInputId)> {
    let mut bad_connections: Vec<(SoundNumberSourceId, SoundNumberInputId)> = Vec::new();

    for (niid, ni) in topology.number_inputs() {
        for target in ni.target_mapping().items().values() {
            let target_owner = topology.number_source(*target).unwrap().owner();
            let depends = match target_owner {
                SoundNumberSourceOwner::SoundProcessor(spid) => {
                    processor_depends_on_processor(spid, ni.owner(), topology)
                }
                SoundNumberSourceOwner::SoundInput(siid) => {
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

pub(crate) fn validate_sound_connection(
    topology: &SoundGraphTopology,
    input_id: SoundInputId,
    processor_id: SoundProcessorId,
) -> Result<(), SoundError> {
    // Lazy approach: duplicate the topology, make the edit, and see what happens
    let mut topo = topology.clone();
    topo.make_sound_edit(SoundEdit::ConnectSoundInput(input_id, processor_id));
    match find_error(&topo) {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub(crate) fn validate_sound_disconnection(
    topology: &SoundGraphTopology,
    input_id: SoundInputId,
) -> Result<(), SoundError> {
    // Lazy approach: duplicate the topology, make the edit, and see what happens
    let mut topo = topology.clone();
    topo.make_sound_edit(SoundEdit::DisconnectSoundInput(input_id));
    match find_error(&topo) {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

pub(crate) fn validate_sound_number_connection(
    topology: &SoundGraphTopology,
    input_id: SoundNumberInputId,
    source_id: SoundNumberSourceId,
) -> Result<(), SoundError> {
    todo!()
}

pub(crate) fn available_sound_number_sources(
    topology: &SoundGraphTopology,
) -> HashMap<SoundProcessorId, HashSet<SoundNumberSourceId>> {
    let mut cached_proc_sources: HashMap<SoundProcessorId, HashSet<SoundNumberSourceId>> =
        HashMap::new();
    for proc_data in topology.sound_processors().values() {
        if proc_data.instance().is_static() {
            cached_proc_sources.insert(
                proc_data.id(),
                proc_data.number_sources().iter().cloned().collect(),
            );
        }
    }

    let all_targets_cached_for =
        |processor_id: SoundProcessorId,
         cache: &HashMap<SoundProcessorId, HashSet<SoundNumberSourceId>>| {
            topology
                .sound_processor_targets(processor_id)
                .all(|target_siid| {
                    let parent_sp = topology.sound_input(target_siid).unwrap().owner();
                    cache.contains_key(&parent_sp)
                })
        };

    let sound_input_number_sources =
        |input_id: SoundInputId,
         cache: &HashMap<SoundProcessorId, HashSet<SoundNumberSourceId>>|
         -> HashSet<SoundNumberSourceId> {
            let input_data = topology.sound_input(input_id).unwrap();
            let mut sources = cache
                .get(&input_data.owner())
                .expect("Processor number sources should have been cached")
                .clone();
            for nsid in input_data.number_sources() {
                sources.insert(*nsid);
            }
            sources
        };

    loop {
        let next_proc_id = topology
            .sound_processors()
            .values()
            .filter_map(|proc_data| {
                // don't revisit processors that are already cached
                if cached_proc_sources.contains_key(&proc_data.id()) {
                    return None;
                }
                // visit processors for which all targets are cached
                if all_targets_cached_for(proc_data.id(), &cached_proc_sources) {
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

        let mut available_sources: Option<HashSet<SoundNumberSourceId>> = None;

        // Available upstream sources are the intersection of all those
        // available via each destination sound input
        for target_input in topology.sound_processor_targets(next_proc_id) {
            let target_input_sources =
                sound_input_number_sources(target_input, &cached_proc_sources);
            if let Some(sources) = available_sources.as_mut() {
                *sources = sources
                    .intersection(&target_input_sources)
                    .cloned()
                    .collect();
            } else {
                available_sources = Some(target_input_sources);
            }
        }

        let mut available_sources = available_sources.unwrap_or_else(HashSet::new);

        for nsid in topology
            .sound_processor(next_proc_id)
            .unwrap()
            .number_sources()
        {
            available_sources.insert(*nsid);
        }

        cached_proc_sources.insert(next_proc_id, available_sources);
    }

    cached_proc_sources
}
