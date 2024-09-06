use super::{
    expressiongraph::ExpressionGraph, expressiongraphdata::ExpressionTarget,
    expressiongrapherror::ExpressionError, expressionnodeinput::ExpressionNodeInputId,
    path::ExpressionPath,
};

pub(super) fn find_expression_error(topology: &ExpressionGraph) -> Option<ExpressionError> {
    check_missing_ids(topology);

    if let Some(path) = find_expression_cycle(topology) {
        return Some(ExpressionError::CircularDependency { cycle: path });
    }

    None
}

fn check_missing_ids(topology: &ExpressionGraph) {
    for ns in topology.nodes().values() {
        // for each node

        for ni in ns.inputs() {
            // each node input must list the node as its owner
            match topology.node_input(*ni) {
                Some(nidata) => {
                    if nidata.owner() != ns.id() {
                        panic!(
                            "Node {:?} has input {:?} listed as an input, \
                            but that input does not list the node as its owner.",
                            ns.id(),
                            *ni
                        );
                    }
                }
                None => panic!(
                    "Node {:?} has input {:?} listed as an input, \
                    but that input does not exist.",
                    ns.id(),
                    *ni
                ),
            }
        }
    }

    for ni in topology.node_inputs().values() {
        // for each node input

        // its owner must exist
        if topology.node(ni.owner()).is_none() {
            panic!(
                "Node input {:?} lists node {:?} as its owner, but \
                that node does not exist.",
                ni.id(),
                ni.owner()
            );
        }

        // its target, if any, must exist
        match ni.target() {
            Some(ExpressionTarget::Node(nsid)) => {
                if topology.node(nsid).is_none() {
                    panic!(
                        "Node input {:?} lists node {:?} as its target, \
                        but that node does not exist.",
                        ni.id(),
                        nsid
                    );
                }
            }
            Some(ExpressionTarget::Parameter(giid)) => {
                if !topology.parameters().contains(&giid) {
                    panic!(
                        "Node input {:?} lists graph input {:?} as its target, \
                        but that graph input does not exist.",
                        ni.id(),
                        giid
                    );
                }
            }
            None => (),
        }
    }

    for go in topology.results() {
        // for each graph output

        // its target, if any, must exist
        match go.target() {
            Some(ExpressionTarget::Node(nsid)) => {
                if topology.node(nsid).is_none() {
                    panic!(
                        "Graph output {:?} lists node {:?} as its target, \
                        but that node does not exist.",
                        go.id(),
                        nsid
                    );
                }
            }
            Some(ExpressionTarget::Parameter(giid)) => {
                if !topology.parameters().contains(&giid) {
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

fn find_expression_cycle(topology: &ExpressionGraph) -> Option<ExpressionPath> {
    fn dfs_find_cycle(
        input_id: ExpressionNodeInputId,
        visited: &mut Vec<ExpressionNodeInputId>,
        path: &mut ExpressionPath,
        topo: &ExpressionGraph,
    ) -> Option<ExpressionPath> {
        if !visited.contains(&input_id) {
            visited.push(input_id);
        }
        // If the input has already been visited, there is a cycle
        if path.contains_input(input_id) {
            return Some(path.trim_until_input(input_id));
        }
        let input_desc = topo.node_input(input_id).unwrap();
        let Some(ExpressionTarget::Node(target_id)) = input_desc.target() else {
            return None;
        };
        let proc_desc = topo.node(target_id).unwrap();
        path.push(target_id, input_id);
        for target_proc_input in proc_desc.inputs() {
            if let Some(path) = dfs_find_cycle(*target_proc_input, visited, path, topo) {
                return Some(path);
            }
        }
        path.pop();
        None
    }

    let mut visited: Vec<ExpressionNodeInputId> = Vec::new();
    let mut path = ExpressionPath::new(Vec::new());

    loop {
        assert_eq!(path.connections.len(), 0);
        let input_to_visit = topology
            .node_inputs()
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

pub(crate) fn validate_expression_connection(
    topology: &ExpressionGraph,
    input_id: ExpressionNodeInputId,
    target: ExpressionTarget,
) -> Result<(), ExpressionError> {
    // Lazy approach: duplicate the topology, make the edit, and see what happens
    let mut topology = topology.clone();
    topology.connect_node_input(input_id, target)?;
    if let Some(err) = find_expression_error(&topology) {
        return Err(err);
    }
    Ok(())
}
