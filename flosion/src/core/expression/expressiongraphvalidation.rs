use std::collections::HashSet;

use crate::core::expression::{
    expressiongraphdata::ExpressionTarget, expressioninput::ExpressionInputLocation,
    expressionnode::ExpressionNodeId,
};

use super::{expressiongraph::ExpressionGraph, expressiongrapherror::ExpressionError};

pub(crate) fn find_expression_error(graph: &ExpressionGraph) -> Option<ExpressionError> {
    if find_expression_cycle(graph) {
        return Some(ExpressionError::CircularDependency);
    }

    None
}

fn find_expression_cycle(graph: &ExpressionGraph) -> bool {
    fn find_cycle(
        node_id: ExpressionNodeId,
        visited_inputs: &mut HashSet<ExpressionInputLocation>,
        graph: &ExpressionGraph,
    ) -> bool {
        let mut any_cycles = false;

        let mut queue = vec![node_id];

        while !queue.is_empty() && !any_cycles {
            let node_id = queue.remove(0);
            graph
                .node(node_id)
                .unwrap()
                .foreach_input(|input, location| {
                    if visited_inputs.contains(&location) {
                        any_cycles = true;
                        return;
                    }
                    visited_inputs.insert(location);
                    if let Some(ExpressionTarget::Node(target_id)) = input.target() {
                        queue.push(target_id);
                    }
                });
        }
        any_cycles
    }

    let mut visited_nodes: HashSet<ExpressionNodeId> = HashSet::new();

    loop {
        let Some(node_to_visit) = graph
            .nodes()
            .keys()
            .find(|pid| !visited_nodes.contains(&pid))
            .cloned()
        else {
            return false;
        };
        let mut visited_inputs = HashSet::new();
        if find_cycle(node_to_visit, &mut visited_inputs, graph) {
            return true;
        }
        visited_nodes.insert(node_to_visit);
        for input_location in visited_inputs {
            if let ExpressionInputLocation::NodeInput(node_id, _) = input_location {
                visited_nodes.insert(node_id);
            }
        }
    }
}
