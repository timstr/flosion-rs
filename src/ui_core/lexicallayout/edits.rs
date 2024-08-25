use core::panic;

use crate::{
    core::{
        expression::{
            expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
            expressiongraphdata::ExpressionTarget,
        },
        sound::soundgraph::SoundGraph,
    },
    ui_core::{
        expressiongraphuicontext::OuterExpressionGraphUiContext,
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
    outer_context: &OuterExpressionGraphUiContext,
    sound_graph: &mut SoundGraph,
) {
    delete_nodes_from_graph_at_cursor(cursor, layout, outer_context, sound_graph);

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
    outer_context: &OuterExpressionGraphUiContext,
    sound_graph: &mut SoundGraph,
) {
    // TODO: allow inserting operators in-place (e.g. wrap a value in a function call?)
    delete_from_graph_at_cursor(layout, cursor, outer_context, sound_graph);

    if let Some(target) = node.indirect_target(cursor.get_variables_in_scope(layout)) {
        let (root_node, path) = match cursor.get(layout) {
            LexicalLayoutCursorValue::AtVariableName(_) => {
                panic!("Can't insert over a variable's name")
            }
            LexicalLayoutCursorValue::AtVariableValue(v, p) => {
                if p.is_at_beginning() {
                    // if the cursor points to a variable definition, reconnect each use
                    connect_each_variable_use(layout, v.id(), target, outer_context, sound_graph);
                }
                (v.value(), p)
            }
            LexicalLayoutCursorValue::AtFinalExpression(n, p) => {
                if p.is_at_beginning() {
                    // if the cursor points to the final expression, reconnect
                    // the graph output
                    outer_context
                        .edit_expression_graph(sound_graph, |graph| {
                            // if the cursor points to the final expression, reconnect
                            // the graph output
                            let results = graph.topology().results();
                            debug_assert_eq!(results.len(), 1);
                            let result = results.first().unwrap();
                            debug_assert_eq!(n.direct_target(), None);
                            graph.connect_result(result.id(), target).unwrap();
                        })
                        .unwrap();
                }
                (n, p)
            }
        };

        // if the cursor points to an ordinary internal node, reconnect
        // just its parent
        if let Some((parent_node, child_index)) = root_node.find_parent_along_path(path.steps()) {
            outer_context
                .edit_expression_graph(sound_graph, |graph| {
                    let parent_nsid = parent_node.expression_node_id();
                    let parent_ns = graph.topology().node(parent_nsid).unwrap();
                    let parent_inputs = parent_ns.inputs();
                    debug_assert_eq!(parent_inputs.len(), parent_node.num_children());
                    let input_id = parent_inputs[child_index];
                    graph.connect_node_input(input_id, target).unwrap();
                })
                .unwrap();
        }
    }

    cursor.set_node(layout, node);
}

fn delete_nodes_from_graph_at_cursor(
    cursor: &LexicalLayoutCursor,
    layout: &mut LexicalLayout,
    outer_context: &OuterExpressionGraphUiContext,
    sound_graph: &mut SoundGraph,
) {
    fn remove_node(node: &ASTNode, graph: &mut ExpressionGraph) {
        if let Some(internal_node) = node.as_internal_node() {
            remove_internal_node(internal_node, graph);
        }
    }

    fn remove_internal_node(node: &InternalASTNode, graph: &mut ExpressionGraph) {
        let nsid = node.expression_node_id();

        // Recursively delete any expression nodes corresponding to direct AST children
        match node.value() {
            InternalASTNodeValue::Prefix(_, c) => {
                remove_node(c, graph);
            }
            InternalASTNodeValue::Infix(c1, _, c2) => {
                remove_node(c1, graph);
                remove_node(c2, graph);
            }
            InternalASTNodeValue::Postfix(c, _) => {
                remove_node(c, graph);
            }
            InternalASTNodeValue::Function(_, cs) => {
                for c in cs {
                    remove_node(c, graph);
                }
            }
        }

        // Delete the expression node itself
        graph.remove_expression_node(nsid).unwrap();
    }

    outer_context
        .edit_expression_graph(sound_graph, |graph| {
            let (root_node, path) = match cursor.get(layout) {
                LexicalLayoutCursorValue::AtVariableName(v) => {
                    disconnect_each_variable_use(layout, cursor, v.id(), graph);
                    (v.value(), ASTPath::new_at_beginning())
                }
                LexicalLayoutCursorValue::AtVariableValue(v, p) => {
                    if p.is_at_beginning() {
                        disconnect_each_variable_use(layout, cursor, v.id(), graph);
                    }
                    (v.value(), p)
                }
                LexicalLayoutCursorValue::AtFinalExpression(n, p) => {
                    if p.is_at_beginning() {
                        disconnect_result(layout, graph);
                    }
                    (n, p)
                }
            };

            if let Some((parent_node, child_index)) = root_node.find_parent_along_path(path.steps())
            {
                disconnect_internal_node(parent_node, child_index, graph);
            }

            // If the node is an internal node, disconnect and recursively delete it
            if let Some(internal_node) = root_node.get_along_path(path.steps()).as_internal_node() {
                remove_internal_node(internal_node, graph);
            }

            // If the cursor is pointing at a variable's name, delete all references to that variable
            if let LexicalLayoutCursor::AtVariableName(i) = cursor {
                let var_id = layout.variable_definitions()[*i].id();
                delete_matching_variable_nodes_from_layout(layout, var_id);
            }
        })
        .unwrap();
}

fn disconnect_each_variable_use(
    layout: &LexicalLayout,
    cursor: &LexicalLayoutCursor,
    id: VariableId,
    graph: &mut ExpressionGraph,
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
                disconnect_each_variable_use(layout, cursor, var_id, graph);
            }
            ASTNodeParent::FinalExpression => {
                let outputs = graph.topology().results();
                debug_assert_eq!(outputs.len(), 1);
                let goid = outputs[0].id();
                graph.disconnect_result(goid).unwrap();
            }
            ASTNodeParent::InternalNode(internal_node, child_index) => {
                let nsid = internal_node.expression_node_id();
                let inputs = graph.topology().node(nsid).unwrap().inputs();
                debug_assert_eq!(inputs.len(), internal_node.num_children());
                let niid = inputs[child_index];
                graph.disconnect_node_input(niid).unwrap();
            }
        }
    });
}

fn connect_each_variable_use(
    layout: &LexicalLayout,
    id: VariableId,
    target: ExpressionTarget,
    outer_context: &OuterExpressionGraphUiContext,
    sound_graph: &mut SoundGraph,
) {
    let mut variables_to_connect = vec![id];

    while let Some(id) = variables_to_connect.pop() {
        outer_context
            .edit_expression_graph(sound_graph, |graph| {
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
                            let outputs = graph.topology().results();
                            debug_assert_eq!(outputs.len(), 1);
                            let goid = outputs[0].id();
                            graph.connect_result(goid, target).unwrap();
                        }
                        ASTNodeParent::InternalNode(internal_node, child_index) => {
                            let nsid = internal_node.expression_node_id();
                            let inputs = graph.topology().node(nsid).unwrap().inputs();
                            debug_assert_eq!(inputs.len(), internal_node.num_children());
                            let niid = inputs[child_index];
                            graph.connect_node_input(niid, target).unwrap();
                        }
                    }
                });
            })
            .unwrap();
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
    outer_context: &OuterExpressionGraphUiContext,
    sound_graph: &mut SoundGraph,
) {
    let mut referenced_parameters = Vec::<ExpressionGraphParameterId>::new();

    let all_parameters = outer_context.inspect_expression_graph(sound_graph.topology(), |graph| {
        layout.visit(|node, _path| {
            if let ASTNodeValue::Parameter(giid) = node.value() {
                if !referenced_parameters.contains(&giid) {
                    referenced_parameters.push(*giid);
                }
            }
        });

        debug_assert!((|| {
            for giid in &referenced_parameters {
                if !graph.topology().parameters().contains(giid) {
                    return false;
                }
            }
            true
        })());

        let all_parameters = graph.topology().parameters().to_vec();
        all_parameters
    });

    for giid in all_parameters {
        if !referenced_parameters.contains(&giid) {
            outer_context.remove_parameter(sound_graph, giid);
        }
    }
}

fn disconnect_result(layout: &LexicalLayout, graph: &mut ExpressionGraph) {
    let results = graph.topology().results();
    debug_assert_eq!(results.len(), 1);
    let result = results.first().unwrap();
    debug_assert_eq!(
        layout
            .final_expression()
            .indirect_target(layout.variable_definitions()),
        result.target()
    );
    if graph
        .topology()
        .result(result.id())
        .unwrap()
        .target()
        .is_none()
    {
        // Graph output is already disconnected
        return;
    }
    graph.disconnect_result(result.id()).unwrap();
}

fn disconnect_internal_node(
    parent_node: &InternalASTNode,
    child_index: usize,
    graph: &mut ExpressionGraph,
) {
    let nsid = parent_node.expression_node_id();
    let parent_ns_inputs = graph.topology().node(nsid).unwrap().inputs();
    debug_assert_eq!(parent_ns_inputs.len(), parent_node.num_children());
    let input = parent_ns_inputs[child_index];
    if graph
        .topology()
        .node_input(input)
        .unwrap()
        .target()
        .is_some()
    {
        graph.disconnect_node_input(input).unwrap();
    }
}
