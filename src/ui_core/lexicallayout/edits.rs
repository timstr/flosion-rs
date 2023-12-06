use core::panic;

use crate::{
    core::number::{
        numbergraph::{NumberGraph, NumberGraphInputId},
        numbergraphdata::NumberTarget,
    },
    ui_core::{
        lexicallayout::ast::{ASTPath, InternalASTNodeValue},
        numbergraphuicontext::OuterNumberGraphUiContext,
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

pub(super) fn delete_from_numbergraph_at_cursor(
    layout: &mut LexicalLayout,
    cursor: &mut LexicalLayoutCursor,
    outer_context: &mut OuterNumberGraphUiContext,
) {
    delete_ast_nodes_from_graph(cursor, layout, outer_context);

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

pub(super) fn insert_to_numbergraph_at_cursor(
    layout: &mut LexicalLayout,
    cursor: &mut LexicalLayoutCursor,
    node: ASTNode,
    outer_context: &mut OuterNumberGraphUiContext,
) {
    // TODO: allow inserting operators in-place
    delete_from_numbergraph_at_cursor(layout, cursor, outer_context);

    if let Some(target) = node.indirect_target(cursor.get_variables_in_scope(layout)) {
        let (root_node, path) = match cursor.get(layout) {
            LexicalLayoutCursorValue::AtVariableName(_) => {
                panic!("Can't insert over a variable's name")
            }
            LexicalLayoutCursorValue::AtVariableValue(v, p) => {
                if p.is_at_beginning() {
                    // if the cursor points to a variable definition, reconnect each use
                    connect_each_variable_use(layout, v.id(), target, outer_context);
                }
                (v.value(), p)
            }
            LexicalLayoutCursorValue::AtFinalExpression(n, p) => {
                if p.is_at_beginning() {
                    // if the cursor points to the final expression, reconnect
                    // the graph output
                    outer_context
                        .edit_number_graph(|numbergraph| {
                            // if the cursor points to the final expression, reconnect
                            // the graph output
                            let graph_outputs = numbergraph.topology().graph_outputs();
                            debug_assert_eq!(graph_outputs.len(), 1);
                            let graph_output = graph_outputs.first().unwrap();
                            debug_assert_eq!(n.direct_target(), None);
                            numbergraph
                                .connect_graph_output(graph_output.id(), target)
                                .unwrap();
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
                .edit_number_graph(|numbergraph| {
                    let parent_nsid = parent_node.number_source_id();
                    let parent_ns = numbergraph.topology().number_source(parent_nsid).unwrap();
                    let parent_inputs = parent_ns.number_inputs();
                    debug_assert_eq!(parent_inputs.len(), parent_node.num_children());
                    let input_id = parent_inputs[child_index];
                    numbergraph.connect_number_input(input_id, target).unwrap();
                })
                .unwrap();
        }
    }

    cursor.set_node(layout, node);
}

fn delete_ast_nodes_from_graph(
    cursor: &LexicalLayoutCursor,
    layout: &mut LexicalLayout,
    outer_context: &mut OuterNumberGraphUiContext,
) {
    fn remove_node(node: &ASTNode, numbergraph: &mut NumberGraph) {
        if let Some(internal_node) = node.as_internal_node() {
            remove_internal_node(internal_node, numbergraph);
        }
    }

    fn remove_internal_node(node: &InternalASTNode, numbergraph: &mut NumberGraph) {
        let nsid = node.number_source_id();

        // Recursively delete any number sources corresponding to direct AST children
        match node.value() {
            InternalASTNodeValue::Prefix(_, c) => {
                remove_node(c, numbergraph);
            }
            InternalASTNodeValue::Infix(c1, _, c2) => {
                remove_node(c1, numbergraph);
                remove_node(c2, numbergraph);
            }
            InternalASTNodeValue::Postfix(c, _) => {
                remove_node(c, numbergraph);
            }
            InternalASTNodeValue::Function(_, cs) => {
                for c in cs {
                    remove_node(c, numbergraph);
                }
            }
        }

        // Delete the number source itself
        numbergraph.remove_number_source(nsid).unwrap();
    }

    outer_context
        .edit_number_graph(|numbergraph| {
            let (root_node, path) = match cursor.get(layout) {
                LexicalLayoutCursorValue::AtVariableName(v) => {
                    disconnect_each_variable_use(layout, cursor, v.id(), numbergraph);
                    (v.value(), ASTPath::new_at_beginning())
                }
                LexicalLayoutCursorValue::AtVariableValue(v, p) => {
                    if p.is_at_beginning() {
                        disconnect_each_variable_use(layout, cursor, v.id(), numbergraph);
                    }
                    (v.value(), p)
                }
                LexicalLayoutCursorValue::AtFinalExpression(n, p) => {
                    if p.is_at_beginning() {
                        disconnect_graph_output(layout, numbergraph);
                    }
                    (n, p)
                }
            };

            if let Some((parent_node, child_index)) = root_node.find_parent_along_path(path.steps())
            {
                disconnect_internal_node(parent_node, child_index, numbergraph);
            }

            // If the node is an internal node, disconnect and recursively delete it
            if let Some(internal_node) = root_node.get_along_path(path.steps()).as_internal_node() {
                remove_internal_node(internal_node, numbergraph);
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
    numbergraph: &mut NumberGraph,
) {
    let (var_defn, prev_defns) =
        find_variable_definition_and_scope(id, layout.variable_definitions()).unwrap();

    if var_defn.value().indirect_target(prev_defns).is_none() {
        // The variable doesn't correspond to any number graph target, and so
        // all ASTNodes pointing to it should already represent empty number
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
                disconnect_each_variable_use(layout, cursor, var_id, numbergraph);
            }
            ASTNodeParent::FinalExpression => {
                let outputs = numbergraph.topology().graph_outputs();
                debug_assert_eq!(outputs.len(), 1);
                let goid = outputs[0].id();
                numbergraph.disconnect_graph_output(goid).unwrap();
            }
            ASTNodeParent::InternalNode(internal_node, child_index) => {
                let nsid = internal_node.number_source_id();
                let number_inputs = numbergraph
                    .topology()
                    .number_source(nsid)
                    .unwrap()
                    .number_inputs();
                debug_assert_eq!(number_inputs.len(), internal_node.num_children());
                let niid = number_inputs[child_index];
                numbergraph.disconnect_number_input(niid).unwrap();
            }
        }
    });
}

fn connect_each_variable_use(
    layout: &LexicalLayout,
    id: VariableId,
    target: NumberTarget,
    outer_context: &mut OuterNumberGraphUiContext,
) {
    let mut variables_to_connect = vec![id];

    while let Some(id) = variables_to_connect.pop() {
        outer_context
            .edit_number_graph(|numbergraph| {
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
                            let outputs = numbergraph.topology().graph_outputs();
                            debug_assert_eq!(outputs.len(), 1);
                            let goid = outputs[0].id();
                            numbergraph.connect_graph_output(goid, target).unwrap();
                        }
                        ASTNodeParent::InternalNode(internal_node, child_index) => {
                            let nsid = internal_node.number_source_id();
                            let number_inputs = numbergraph
                                .topology()
                                .number_source(nsid)
                                .unwrap()
                                .number_inputs();
                            debug_assert_eq!(number_inputs.len(), internal_node.num_children());
                            let niid = number_inputs[child_index];
                            numbergraph.connect_number_input(niid, target).unwrap();
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

pub(super) fn remove_unreferenced_graph_inputs(
    layout: &LexicalLayout,
    outer_context: &mut OuterNumberGraphUiContext,
) {
    let mut referenced_graph_inputs = Vec::<NumberGraphInputId>::new();

    let all_graph_inputs = outer_context.inspect_number_graph(|numbergraph| {
        layout.visit(|node, _path| {
            if let ASTNodeValue::GraphInput(giid) = node.value() {
                if !referenced_graph_inputs.contains(&giid) {
                    referenced_graph_inputs.push(*giid);
                }
            }
        });

        debug_assert!((|| {
            for giid in &referenced_graph_inputs {
                if !numbergraph.topology().graph_inputs().contains(giid) {
                    return false;
                }
            }
            true
        })());

        let all_graph_inputs = numbergraph.topology().graph_inputs().to_vec();
        all_graph_inputs
    });

    for giid in all_graph_inputs {
        if !referenced_graph_inputs.contains(&giid) {
            outer_context.remove_graph_input(giid);
        }
    }
}

fn disconnect_graph_output(layout: &LexicalLayout, numbergraph: &mut NumberGraph) {
    let graph_outputs = numbergraph.topology().graph_outputs();
    debug_assert_eq!(graph_outputs.len(), 1);
    let graph_output = graph_outputs.first().unwrap();
    debug_assert_eq!(
        layout
            .final_expression()
            .indirect_target(layout.variable_definitions()),
        graph_output.target()
    );
    if numbergraph
        .topology()
        .graph_output(graph_output.id())
        .unwrap()
        .target()
        .is_none()
    {
        // Graph output is already disconnected
        return;
    }
    numbergraph
        .disconnect_graph_output(graph_output.id())
        .unwrap();
}

fn disconnect_internal_node(
    parent_node: &InternalASTNode,
    child_index: usize,
    numbergraph: &mut NumberGraph,
) {
    let nsid = parent_node.number_source_id();
    let parent_ns_inputs = numbergraph
        .topology()
        .number_source(nsid)
        .unwrap()
        .number_inputs();
    debug_assert_eq!(parent_ns_inputs.len(), parent_node.num_children());
    let input = parent_ns_inputs[child_index];
    if numbergraph
        .topology()
        .number_input(input)
        .unwrap()
        .target()
        .is_some()
    {
        numbergraph.disconnect_number_input(input).unwrap();
    }
}
