use core::panic;

use hashstash::Stash;

use crate::{
    core::expression::{
        expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
        expressiongraphdata::ExpressionTarget,
    },
    ui_core::{
        expressiongraphuicontext::OuterExpressionGraphUiContext,
        factories::Factories,
        lexicallayout::ast::{ASTPath, InternalASTNodeValue},
    },
};

use super::{
    ast::{
        find_variable_definition_and_scope, ASTNode, ASTNodeParent, ASTNodeValue, InternalASTNode,
        VariableId,
    },
    cursor::{LexicalLayoutCursor, LexicalLayoutCursorValue},
    lexicallayout::LexicalLayout,
};

pub(super) fn delete_from_graph_at_cursor(
    layout: &mut LexicalLayout,
    cursor: &mut LexicalLayoutCursor,
    expr_graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    delete_nodes_from_graph_at_cursor(cursor, layout, expr_graph, stash, factories);

    match cursor {
        LexicalLayoutCursor::AtVariableName(i) => {
            layout.variable_definitions_mut().remove(*i);
            if *i == layout.variable_definitions().len() {
                cursor.go_to_final_expression();
            }
        }
        LexicalLayoutCursor::AtVariableValue(i, p) => {
            layout.variable_definitions_mut()[*i]
                .value_mut()
                .set_along_path(p.steps(), ASTNode::new(ASTNodeValue::Empty));
            cursor.set_node(layout, ASTNode::new(ASTNodeValue::Empty));
        }
        LexicalLayoutCursor::AtFinalExpression(p) => {
            layout
                .final_expression_mut()
                .set_along_path(p.steps(), ASTNode::new(ASTNodeValue::Empty));
        }
    }
}

pub(super) fn insert_to_graph_at_cursor(
    layout: &mut LexicalLayout,
    cursor: &mut LexicalLayoutCursor,
    node: ASTNode,
    expr_graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    // TODO: allow inserting operators in-place (e.g. wrap a value in a function call?)
    delete_from_graph_at_cursor(layout, cursor, expr_graph, stash, factories);

    if let Some(target) = node.indirect_target(cursor.get_variables_in_scope(layout)) {
        let (root_node, path) = match cursor.get(layout) {
            LexicalLayoutCursorValue::AtVariableName(_) => {
                panic!("Can't insert over a variable's name")
            }
            LexicalLayoutCursorValue::AtVariableValue(v, p) => {
                if p.is_at_beginning() {
                    // if the cursor points to a variable definition, reconnect each use
                    connect_each_variable_use(layout, v.id(), target, expr_graph, stash, factories);
                }
                (v.value(), p)
            }
            LexicalLayoutCursorValue::AtFinalExpression(n, p) => {
                if p.is_at_beginning() {
                    // if the cursor points to the final expression, reconnect
                    // the graph output

                    let results = expr_graph.results();
                    debug_assert_eq!(results.len(), 1);
                    let result = results.first().unwrap();
                    debug_assert_eq!(n.direct_target(), None);
                    expr_graph.connect_result(result.id(), target).unwrap();
                }
                (n, p)
            }
        };

        // if the cursor points to an ordinary internal node, reconnect
        // just its parent
        if let Some((parent_node, child_index)) = root_node.find_parent_along_path(path.steps()) {
            let parent_nsid = parent_node.expression_node_id();
            let parent_ns = expr_graph.node(parent_nsid).unwrap();
            let parent_inputs = parent_ns.input_locations();
            debug_assert_eq!(parent_inputs.len(), parent_node.num_children());
            let input_id = parent_inputs[child_index];
            expr_graph.connect_input(input_id, Some(target)).unwrap();
        }
    }

    cursor.set_node(layout, node);
}

fn delete_nodes_from_graph_at_cursor(
    cursor: &LexicalLayoutCursor,
    layout: &mut LexicalLayout,
    expr_graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    fn remove_node(
        node: &ASTNode,
        graph: &mut ExpressionGraph,
        stash: &Stash,
        factories: &Factories,
    ) {
        if let Some(internal_node) = node.as_internal_node() {
            remove_internal_node(internal_node, graph, stash, factories);
        }
    }

    fn remove_internal_node(
        node: &InternalASTNode,
        graph: &mut ExpressionGraph,
        stash: &Stash,
        factories: &Factories,
    ) {
        let nsid = node.expression_node_id();

        // Recursively delete any expression nodes corresponding to direct AST children
        match node.value() {
            InternalASTNodeValue::Prefix(_, c) => {
                remove_node(c, graph, stash, factories);
            }
            InternalASTNodeValue::Infix(c1, _, c2) => {
                remove_node(c1, graph, stash, factories);
                remove_node(c2, graph, stash, factories);
            }
            InternalASTNodeValue::Postfix(c, _) => {
                remove_node(c, graph, stash, factories);
            }
            InternalASTNodeValue::Function(_, cs) => {
                for c in cs {
                    remove_node(c, graph, stash, factories);
                }
            }
        }

        // Delete the expression node itself
        graph
            .try_make_change(stash, factories, |graph| graph.remove_node(nsid))
            .unwrap();
    }

    let (root_node, path) = match cursor.get(layout) {
        LexicalLayoutCursorValue::AtVariableName(v) => {
            disconnect_each_variable_use(layout, cursor, v.id(), expr_graph, stash, factories);
            (v.value(), ASTPath::new_at_beginning())
        }
        LexicalLayoutCursorValue::AtVariableValue(v, p) => {
            if p.is_at_beginning() {
                disconnect_each_variable_use(layout, cursor, v.id(), expr_graph, stash, factories);
            }
            (v.value(), p)
        }
        LexicalLayoutCursorValue::AtFinalExpression(n, p) => {
            if p.is_at_beginning() {
                disconnect_result(layout, expr_graph, stash, factories);
            }
            (n, p)
        }
    };

    if let Some((parent_node, child_index)) = root_node.find_parent_along_path(path.steps()) {
        disconnect_internal_node(parent_node, child_index, expr_graph, stash, factories);
    }

    // If the node is an internal node, disconnect and recursively delete it
    if let Some(internal_node) = root_node.get_along_path(path.steps()).as_internal_node() {
        remove_internal_node(internal_node, expr_graph, stash, factories);
    }

    // If the cursor is pointing at a variable's name, delete all references to that variable
    if let LexicalLayoutCursor::AtVariableName(i) = cursor {
        let var_id = layout.variable_definitions()[*i].id();
        delete_matching_variable_nodes_from_layout(layout, var_id);
    }
}

fn disconnect_each_variable_use(
    layout: &LexicalLayout,
    cursor: &LexicalLayoutCursor,
    id: VariableId,
    graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    let (var_defn, prev_defns) =
        find_variable_definition_and_scope(id, layout.variable_definitions()).unwrap();

    if var_defn.value().indirect_target(prev_defns).is_none() {
        // The variable doesn't correspond to any expression graph target, and so
        // all ASTNodes pointing to it should already represent empty expresion node
        // inputs. Nothing needs to change.
        return;
    }

    layout.visit(|node, path| {
        let ASTNodeValue::Variable(node_id) = node.value() else {
            return;
        };
        let node_id = *node_id;
        if id != node_id {
            return;
        }
        // The node directly references the variable
        match path.parent_node() {
            ASTNodeParent::VariableDefinition(var_id) => {
                debug_assert_ne!(var_id, id);
                // The variable is aliased as another variable, disconnect its uses as well
                disconnect_each_variable_use(layout, cursor, var_id, graph, stash, factories);
            }
            ASTNodeParent::FinalExpression => {
                let outputs = graph.results();
                debug_assert_eq!(outputs.len(), 1);
                let goid = outputs[0].id();
                graph
                    .try_make_change(stash, factories, |graph| graph.disconnect_result(goid))
                    .unwrap();
            }
            ASTNodeParent::InternalNode(internal_node, child_index) => {
                let nsid = internal_node.expression_node_id();
                let inputs = graph.node(nsid).unwrap().input_locations();
                debug_assert_eq!(inputs.len(), internal_node.num_children());
                let niid = inputs[child_index];
                graph
                    .try_make_change(stash, factories, |graph| graph.connect_input(niid, None))
                    .unwrap();
            }
        }
    });
}

fn connect_each_variable_use(
    layout: &LexicalLayout,
    id: VariableId,
    target: ExpressionTarget,
    expr_graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    let mut variables_to_connect = vec![id];

    while let Some(id) = variables_to_connect.pop() {
        layout.visit(|node, path| {
            let ASTNodeValue::Variable(node_id) = node.value() else {
                return;
            };
            let node_id = *node_id;
            if node_id != id {
                return;
            }
            // The node directly references the variable
            match path.parent_node() {
                ASTNodeParent::VariableDefinition(var_id) => {
                    debug_assert_ne!(var_id, id);
                    // The variable is aliased as another variable, disconnect that one too
                    variables_to_connect.push(var_id);
                }
                ASTNodeParent::FinalExpression => {
                    let outputs = expr_graph.results();
                    debug_assert_eq!(outputs.len(), 1);
                    let goid = outputs[0].id();
                    expr_graph
                        .try_make_change(stash, factories, |graph| {
                            graph.connect_result(goid, target)
                        })
                        .unwrap();
                }
                ASTNodeParent::InternalNode(internal_node, child_index) => {
                    let nsid = internal_node.expression_node_id();
                    let inputs = expr_graph.node(nsid).unwrap().input_locations();
                    debug_assert_eq!(inputs.len(), internal_node.num_children());
                    let niid = inputs[child_index];
                    expr_graph
                        .try_make_change(stash, factories, |graph| {
                            graph.connect_input(niid, Some(target))
                        })
                        .unwrap();
                }
            }
        });
    }
}

fn delete_matching_variable_nodes_from_layout(layout: &mut LexicalLayout, id: VariableId) {
    layout.visit_mut(|node, _path| {
        let ASTNodeValue::Variable(vid) = node.value() else {
            return;
        };
        if *vid == id {
            *node = ASTNode::new(ASTNodeValue::Empty);
        }
    });
}

pub(super) fn remove_unreferenced_parameters(
    layout: &LexicalLayout,
    outer_context: &mut OuterExpressionGraphUiContext,
    expr_graph: &mut ExpressionGraph,
) {
    let mut referenced_parameters = Vec::<ExpressionGraphParameterId>::new();

    layout.visit(|node, _path| {
        if let ASTNodeValue::Parameter(giid) = node.value() {
            if !referenced_parameters.contains(&giid) {
                referenced_parameters.push(*giid);
            }
        }
    });

    debug_assert!((|| {
        for giid in &referenced_parameters {
            if !expr_graph.parameters().contains(giid) {
                return false;
            }
        }
        true
    })());

    let all_parameters = expr_graph.parameters().to_vec();

    for giid in all_parameters {
        if !referenced_parameters.contains(&giid) {
            outer_context.remove_parameter(expr_graph, giid);
        }
    }
}

fn disconnect_result(
    layout: &LexicalLayout,
    graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    let results = graph.results();
    debug_assert_eq!(results.len(), 1);
    let result = results.first().unwrap();
    debug_assert_eq!(
        layout
            .final_expression()
            .indirect_target(layout.variable_definitions()),
        result.target()
    );
    let result_id = result.id();
    if graph.result(result_id).unwrap().target().is_none() {
        // Graph output is already disconnected
        return;
    }
    graph
        .try_make_change(stash, factories, |graph| graph.disconnect_result(result_id))
        .unwrap();
}

fn disconnect_internal_node(
    parent_node: &InternalASTNode,
    child_index: usize,
    graph: &mut ExpressionGraph,
    stash: &Stash,
    factories: &Factories,
) {
    let nsid = parent_node.expression_node_id();
    let parent_ns_inputs = graph.node(nsid).unwrap().input_locations();
    debug_assert_eq!(parent_ns_inputs.len(), parent_node.num_children());
    let input = parent_ns_inputs[child_index];
    if graph.input_target(input).unwrap().is_some() {
        graph
            .try_make_change(stash, factories, |graph| graph.connect_input(input, None))
            .unwrap();
    }
}
