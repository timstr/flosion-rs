use std::collections::HashSet;

use crate::{
    core::expression::{
        expressiongraph::ExpressionTarget,
        expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
        expressionnode::ExpressionNodeId,
    },
    ui_core::lexicallayout::ast::{
        find_variable_definition, ASTNode, ASTNodeValue, VariableDefinition,
    },
};

use super::lexicallayout::LexicalLayout;

pub(super) fn lexical_layout_matches_expression_graph(
    layout: &LexicalLayout,
    graph: &ExpressionGraph,
) -> bool {
    let mut visited_sources: HashSet<ExpressionNodeId> = HashSet::new();

    let mut visited_graph_inputs: HashSet<ExpressionGraphParameterId> = HashSet::new();

    fn ast_node_matches_graph(
        node: &ASTNode,
        variables_in_scope: &[VariableDefinition],
        expected_target: Option<ExpressionTarget>,
        visited_sources: &mut HashSet<ExpressionNodeId>,
        visited_graph_inputs: &mut HashSet<ExpressionGraphParameterId>,
        graph: &ExpressionGraph,
    ) -> bool {
        match node.value() {
            ASTNodeValue::Empty => {
                if expected_target.is_some() {
                    println!(
                        "Got an empty ASTNode where {:?} was expected",
                        expected_target
                    );
                    return false;
                }
                return true;
            }
            ASTNodeValue::Internal(inode) => {
                let nsid = inode.expression_node_id();
                let was_inserted = visited_sources.insert(nsid);
                if !was_inserted {
                    println!(
                        "The expression node {} already is represented by a different ASTNode",
                        nsid.value()
                    );
                    return false;
                }
                if expected_target != Some(ExpressionTarget::Node(nsid)) {
                    println!(
                        "Got an internal ASTNode where {:?} was expected",
                        expected_target
                    );
                    return false;
                }
                let ns_data = graph.node(nsid).unwrap();
                let num_ast_children = inode.num_children();
                let ns_inputs = ns_data.input_locations();
                if ns_inputs.len() != num_ast_children {
                    println!(
                        "An internal ASTNode has a different number of inputs from \
                        the expression node it representes"
                    );
                    return false;
                }
                for (i, ns_input) in ns_inputs.iter().cloned().enumerate() {
                    let ast_child = inode.get_child(i);
                    let Ok(ns_input_target) = graph.input_target(ns_input) else {
                        println!(
                            "An internal ASTNode refers to expression node {} which doesn't exist",
                            nsid.value()
                        );
                        return false;
                    };
                    if !ast_node_matches_graph(
                        ast_child,
                        variables_in_scope,
                        ns_input_target,
                        visited_sources,
                        visited_graph_inputs,
                        graph,
                    ) {
                        return false;
                    }
                }
                true
            }
            ASTNodeValue::Variable(v) => {
                if variables_in_scope
                    .iter()
                    .find(|defn| defn.id() == *v)
                    .is_none()
                {
                    println!(
                        "An ASTNode referring to a variable with id {} was found, but no \
                        such variable is defined",
                        v.value()
                    );
                    return false;
                }
                let target = node.indirect_target(variables_in_scope);
                // Don't recurse now, variables will be visited individually
                if target != expected_target {
                    println!(
                        "An ASTNode referring to variable {} \"{}\" was found, but that \
                        variable represents the target {:?} according to the AST while the \
                        target {:?} was expected according to the expression graph",
                        v.value(),
                        find_variable_definition(*v, variables_in_scope)
                            .unwrap()
                            .name(),
                        target,
                        expected_target
                    );
                    return false;
                }
                true
            }
            ASTNodeValue::Parameter(giid) => {
                visited_graph_inputs.insert(*giid);
                if expected_target != Some(ExpressionTarget::Parameter(*giid)) {
                    println!(
                        "An ASTNode pointing to graph input {} was found, but the \
                        expected target is {:?}",
                        giid.value(),
                        expected_target
                    );
                    return false;
                }
                true
            }
        }
    }

    let mut all_good = true;

    if !graph.results().iter().all(|r| {
        layout
            .final_expressions()
            .iter()
            .filter(|fe| fe.result_id == r.id())
            .count()
            == 1
    }) {
        println!("Every result in the expression graph must have a matching final expression");
        all_good = false;
    }

    if !layout
        .final_expressions()
        .iter()
        .all(|fe| graph.result(fe.result_id).is_some())
    {
        println!("Every final expression must have a matching result in the expression graph");
        all_good = false;
    }

    for (i, var_defn) in layout.variable_definitions().iter().enumerate() {
        let variables_in_scope = &layout.variable_definitions()[..i];
        if !ast_node_matches_graph(
            var_defn.value(),
            variables_in_scope,
            var_defn.value().direct_target(),
            &mut visited_sources,
            &mut visited_graph_inputs,
            graph,
        ) {
            println!(
                "Variable definition {} \"{}\" doesn't match the expression graph",
                var_defn.id().value(),
                var_defn.name()
            );
            all_good = false;
        }
    }

    for (i, final_expr) in layout.final_expressions().iter().enumerate() {
        if !ast_node_matches_graph(
            final_expr.value(),
            &layout.variable_definitions(),
            graph.result(final_expr.result_id()).unwrap().target(),
            &mut visited_sources,
            &mut visited_graph_inputs,
            graph,
        ) {
            println!("Final expression {} doesn't match the expression graph", i);
            all_good = false;
        }
    }

    for nsid in graph.nodes().keys() {
        if !visited_sources.contains(nsid) {
            println!(
                "Expression node {} \"{}\" is not represented by any ASTNode",
                nsid.value(),
                graph
                    .node(*nsid)
                    .unwrap()
                    .as_graph_object()
                    .get_dynamic_type()
                    .name()
            );
            all_good = false;
        }
    }

    let graph_inputs = graph.parameters();

    for giid in visited_graph_inputs.iter() {
        if !graph_inputs.contains(giid) {
            println!(
                "A graph input with id {} is referred to by one or more ASTNodes, \
                but no such graph input exists in the expression graph",
                giid.value()
            );
            all_good = false;
        }
    }

    for giid in graph_inputs {
        if !visited_graph_inputs.contains(giid) {
            println!(
                "The expression graph includes a graph input with id {}, but \
                no ASTNode refers to this graph input",
                giid.value()
            );
            all_good = false;
        }
    }

    all_good
}
