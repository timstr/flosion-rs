use std::collections::HashSet;

use crate::core::expression::{
    expressiongraph::ExpressionTarget, expressionnode::ExpressionNodeId,
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
        current_path: &mut Vec<ExpressionNodeId>,
        all_visited_nodes: &mut HashSet<ExpressionNodeId>,
        found_a_cyle: &mut bool,
        graph: &ExpressionGraph,
    ) {
        if current_path.contains(&node_id) {
            *found_a_cyle = true;
            return;
        }
        if all_visited_nodes.contains(&node_id) {
            return;
        }
        all_visited_nodes.insert(node_id);
        graph.node(node_id).unwrap().foreach_input(|input, _| {
            if let Some(ExpressionTarget::Node(target_id)) = input.target() {
                current_path.push(node_id);
                find_cycle(
                    target_id,
                    current_path,
                    all_visited_nodes,
                    found_a_cyle,
                    graph,
                );
                current_path.pop();
            }
        });
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
        let mut path = Vec::new();
        let mut found_a_cycle = false;
        find_cycle(
            node_to_visit,
            &mut path,
            &mut visited_nodes,
            &mut found_a_cycle,
            graph,
        );
        if found_a_cycle {
            return true;
        }
        visited_nodes.insert(node_to_visit);
    }
}
