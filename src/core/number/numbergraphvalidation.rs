use super::{
    numbergraphdata::NumberTarget, numbergraphedit::NumberGraphEdit, numbergrapherror::NumberError,
    numbergraphtopology::NumberGraphTopology, numberinput::NumberInputId, path::NumberPath,
};

pub(crate) fn find_number_error(topology: &NumberGraphTopology) -> Option<NumberError> {
    check_missing_ids(topology);

    if let Some(path) = find_number_cycle(topology) {
        return Some(NumberError::CircularDependency { cycle: path });
    }

    None
}

fn check_missing_ids(topology: &NumberGraphTopology) {
    for ns in topology.number_sources().values() {
        // for each number source

        for ni in ns.number_inputs() {
            // each number input must list the number source as its owner
            match topology.number_input(*ni) {
                Some(nidata) => {
                    if nidata.owner() != ns.id() {
                        panic!(
                            "Number source {:?} has number input {:?} listed as an input, \
                            but that input does not list the number source as its owner.",
                            ns.id(),
                            *ni
                        );
                    }
                }
                None => panic!(
                    "Number source {:?} has number input {:?} listed as an input, \
                    but that input does not exist.",
                    ns.id(),
                    *ni
                ),
            }
        }
    }

    for ni in topology.number_inputs().values() {
        // for each number input

        // its owner must exist
        if topology.number_source(ni.owner()).is_none() {
            panic!(
                "Number input {:?} lists number source {:?} as its owner, but \
                that number source does not exist.",
                ni.id(),
                ni.owner()
            );
        }

        // its target, if any, must exist
        match ni.target() {
            Some(NumberTarget::Source(nsid)) => {
                if topology.number_source(nsid).is_none() {
                    panic!(
                        "Number input {:?} lists number source {:?} as its target, \
                        but that number source does not exist.",
                        ni.id(),
                        nsid
                    );
                }
            }
            Some(NumberTarget::GraphInput(giid)) => {
                if !topology.graph_inputs().contains(&giid) {
                    panic!(
                        "Number input {:?} lists graph input {:?} as its target, \
                        but that graph input does not exist.",
                        ni.id(),
                        giid
                    );
                }
            }
            None => (),
        }
    }

    for go in topology.graph_outputs() {
        // for each graph output

        // its target, if any, must exist
        match go.target() {
            Some(NumberTarget::Source(nsid)) => {
                if topology.number_source(nsid).is_none() {
                    panic!(
                        "Graph output {:?} lists number source {:?} as its target, \
                        but that number source does not exist.",
                        go.id(),
                        nsid
                    );
                }
            }
            Some(NumberTarget::GraphInput(giid)) => {
                if !topology.graph_inputs().contains(&giid) {
                    panic!(
                        "Graph output {:?} lists graph input {:?} as its target, \
                        but that graph input does not exist.",
                        go.id(),
                        giid
                    );
                }
            }
            None => (),
        }
    }

    // no checks needed for graph inputs as they have no additional data
}

fn find_number_cycle(topology: &NumberGraphTopology) -> Option<NumberPath> {
    fn dfs_find_cycle(
        input_id: NumberInputId,
        visited: &mut Vec<NumberInputId>,
        path: &mut NumberPath,
        topo: &NumberGraphTopology,
    ) -> Option<NumberPath> {
        if !visited.contains(&input_id) {
            visited.push(input_id);
        }
        // If the input has already been visited, there is a cycle
        if path.contains_input(input_id) {
            return Some(path.trim_until_input(input_id));
        }
        let input_desc = topo.number_input(input_id).unwrap();
        let Some(NumberTarget::Source(target_id)) = input_desc.target() else {
            return None;
        };
        let proc_desc = topo.number_source(target_id).unwrap();
        path.push(target_id, input_id);
        for target_proc_input in proc_desc.number_inputs() {
            if let Some(path) = dfs_find_cycle(*target_proc_input, visited, path, topo) {
                return Some(path);
            }
        }
        path.pop();
        None
    }

    let mut visited: Vec<NumberInputId> = Vec::new();
    let mut path = NumberPath::new(Vec::new());

    loop {
        assert_eq!(path.connections.len(), 0);
        let input_to_visit = topology
            .number_inputs()
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

pub(crate) fn validate_number_connection(
    topology: &NumberGraphTopology,
    input_id: NumberInputId,
    target: NumberTarget,
) -> Result<(), NumberError> {
    // Lazy approach: duplicate the topology, make the edit, and see what happens
    let mut topology = topology.clone();
    topology.make_edit(NumberGraphEdit::ConnectNumberInput(input_id, target));
    if let Some(err) = find_number_error(&topology) {
        return Err(err);
    }
    Ok(())
}
