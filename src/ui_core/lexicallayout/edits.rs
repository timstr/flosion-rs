use crate::{
    core::number::{
        numbergraph::{NumberGraph, NumberGraphInputId},
        numbergraphdata::NumberTarget,
    },
    ui_core::lexicallayout::ast::InternalASTNodeValue,
};

use super::{
    ast::{ASTNode, ASTNodeParent, ASTNodeValue, ASTRoot, InternalASTNode},
    lexicallayout::{LexicalLayout, LexicalLayoutCursor},
};

pub(super) fn delete_from_numbergraph_at_cursor(
    layout: &mut LexicalLayout,
    cursor: &LexicalLayoutCursor,
    numbergraph: &mut NumberGraph,
) {
    if layout.get_node_at_cursor(cursor).is_empty() {
        return;
    }

    // If the cursor is pointing at a variable definition or the final expression,
    // disconnect those
    match layout.get_cursor_root(cursor) {
        Some(ASTRoot::VariableDefinition(var_def)) => {
            disconnect_each_variable_use(layout, var_def.name(), numbergraph);
        }
        Some(ASTRoot::FinalExpression) => {
            let graph_outputs = numbergraph.topology().graph_outputs();
            debug_assert_eq!(graph_outputs.len(), 1);
            let graph_output = graph_outputs.first().unwrap();
            debug_assert_eq!(
                layout.final_expression().direct_target(),
                graph_output.target()
            );
            numbergraph
                .disconnect_graph_output(graph_output.id())
                .unwrap();
        }
        None => (),
    }

    let node = layout.get_node_at_cursor(cursor);
    if let Some(internal_node) = node.internal_node() {
        delete_internal_node_from_graph(internal_node, numbergraph);
    }
    layout.set_node_at_cursor(cursor, ASTNode::new(ASTNodeValue::Empty));

    remove_unreferenced_graph_inputs(layout, numbergraph);
}

pub(super) fn insert_to_numbergraph_at_cursor(
    layout: &mut LexicalLayout,
    cursor: &mut LexicalLayoutCursor,
    node: ASTNode,
    numbergraph: &mut NumberGraph,
) {
    // TODO: allow inserting operators in-place
    delete_from_numbergraph_at_cursor(layout, cursor, numbergraph);

    if let Some(target) = node.direct_target() {
        match layout.get_cursor_root(cursor) {
            Some(ASTRoot::VariableDefinition(var_def)) => {
                // if the cursor points to a variable definition, reconnect each use
                connect_each_variable_use(layout, var_def.name(), target, numbergraph);
            }
            Some(ASTRoot::FinalExpression) => {
                // if the cursor points to the final expression, reconnect
                // the graph output
                let graph_outputs = numbergraph.topology().graph_outputs();
                debug_assert_eq!(graph_outputs.len(), 1);
                let graph_output = graph_outputs.first().unwrap();
                debug_assert_eq!(layout.final_expression().direct_target(), None);
                numbergraph
                    .connect_graph_output(graph_output.id(), target)
                    .unwrap();
            }
            None => {
                // if the cursor points to an ordinary internal node, reconnect
                // just its parent
                let mut cursor_to_parent = cursor.clone();
                cursor_to_parent.path_mut().go_out();
                let parent_node = layout.get_node_at_cursor(&cursor_to_parent);
                let ASTNodeValue::Internal(parent_node) = parent_node.value() else {
                    panic!()
                };
                let child_index = *cursor.path().steps().last().unwrap();
                let parent_nsid = parent_node.number_source_id();
                let parent_ns = numbergraph.topology().number_source(parent_nsid).unwrap();
                let parent_inputs = parent_ns.number_inputs();
                debug_assert_eq!(parent_inputs.len(), parent_node.num_children());
                let input_id = parent_inputs[child_index];
                numbergraph.connect_number_input(input_id, target).unwrap();
            }
        }
    }

    layout.set_node_at_cursor(cursor, node);
}

fn delete_internal_node_from_graph(node: &InternalASTNode, numbergraph: &mut NumberGraph) {
    let nsid = node.number_source_id();
    let mut dsts = numbergraph
        .topology()
        .number_target_destinations(NumberTarget::Source(nsid));
    let dst = dsts.next();
    // There should only be one thing connected to the number source at this point
    debug_assert!(dsts.next().is_none());
    std::mem::drop(dsts);
    if let Some(dst) = dst {
        numbergraph.disconnect_destination(dst).unwrap();
    };

    fn visit_node(node: &ASTNode, numbergraph: &mut NumberGraph) {
        if let Some(internal_node) = node.internal_node() {
            visitor_internal_node(internal_node, numbergraph);
        }
    }

    fn visitor_internal_node(node: &InternalASTNode, numbergraph: &mut NumberGraph) {
        let nsid = node.number_source_id();

        // Recursively delete any number sources corresponding to direct AST children
        match node.value() {
            InternalASTNodeValue::Prefix(_, c) => {
                visit_node(c, numbergraph);
            }
            InternalASTNodeValue::Infix(c1, _, c2) => {
                visit_node(c1, numbergraph);
                visit_node(c2, numbergraph);
            }
            InternalASTNodeValue::Postfix(c, _) => {
                visit_node(c, numbergraph);
            }
            InternalASTNodeValue::Function(_, cs) => {
                for c in cs {
                    visit_node(c, numbergraph);
                }
            }
        }

        // Delete the number source itself
        numbergraph.remove_number_source(nsid).unwrap();
    }

    visitor_internal_node(node, numbergraph);
}

fn disconnect_each_variable_use(layout: &LexicalLayout, name: &str, numbergraph: &mut NumberGraph) {
    layout.visit(|node, path| {
        let ASTNodeValue::Variable(var_name) = node.value() else {
            return;
        };
        if var_name != name {
            return;
        }
        match path.parent_node() {
            ASTNodeParent::VariableDefinition(var_def) => {
                debug_assert_ne!(var_def.name(), name);
                // FUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUCK aliasing
                panic!();
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
    name: &str,
    target: NumberTarget,
    numbergraph: &mut NumberGraph,
) {
    layout.visit(|node, path| {
        let ASTNodeValue::Variable(var_name) = node.value() else {
            return;
        };
        if var_name != name {
            return;
        }
        match path.parent_node() {
            ASTNodeParent::VariableDefinition(var_def) => {
                debug_assert_ne!(var_def.name(), name);
                // FUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUUCK aliasing
                panic!();
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
}

fn remove_unreferenced_graph_inputs(layout: &LexicalLayout, numbergraph: &mut NumberGraph) {
    let mut referenced_graph_inputs = Vec::<NumberGraphInputId>::new();

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
    for giid in all_graph_inputs {
        if !referenced_graph_inputs.contains(&giid) {
            debug_assert_eq!(
                numbergraph
                    .topology()
                    .number_target_destinations(NumberTarget::GraphInput(giid))
                    .count(),
                0
            );
            // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaa
            // this leaves a dangling entry in the SoundNumberInputData's
            // mapping. BUT, LexicalLayout shouldn't care about the details
            // of SoundNumberInputData since it should work generically for
            // any numbergraph alone (to allow for later top-level number graphs).
            // Removing graph inputs is easy enough (just have the SoundNumberInputData
            // detect and remove its own surplus mapping entry) but adding a new
            // mapping entry is tricky since the SoundNumberInputData can't know
            // which SoundNumberSource it's supposed to be connected to afterwards.
            // Does this imply that some kind of hook/callback is needed to add
            // custom functionality when a number graph input is added via the
            // summon widget? In principle, the summon widget is a great place
            // for this kind of thing since 1) its entries already must differ
            // for sound number inputs in order to account for external sound
            // number sources and 2) this would allow for other functions to
            // be called when a summon widget entry is chosen which might prove
            // useful. For example, in top level number graphs, the same kind
            // of function could add a new named parameter
            // --------------
            // TODO:
            // - remove all references to SoundNumberInputId from LexicalLayout
            // - add a mechanism to LexicalLayout to define a custom summon
            //   widget entry with a custom callback function
            // - use that mechanism in SoundNumberInputUi to add sound number
            //   sources by name to the LexicalLayout's summon widget choices,
            //   make sure that results in new graph inputs being added and
            //   existing graph inputs being reused in agreement with the
            //   SoundNumberInputData's current mapping.
            //    - NOTE that the client-defined function passed to the summon
            //      widget should thus 1) externally connect the sound number
            //      source to the sound number input, possibly obtaining a new
            //      number graph input (a mutable reference passed to the function)
            //      here will help and 2) result in a new graph input being
            //      added to both the AST and the number graph.
            // Where to store the name of the number source? Ideally it should
            // be stored with the LexicalLayout for generality
            numbergraph.remove_graph_input(giid).unwrap();
        }
    }
}
