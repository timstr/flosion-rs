use eframe::egui;

use crate::core::{
    number::{
        numbergraph::NumberGraphInputId, numbergraphdata::NumberTarget,
        numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId,
    },
    uniqueid::UniqueId,
};

use super::numbergraphuicontext::NumberGraphUiContext;

enum ASTNode {
    Empty,
    Full(Box<InternalASTNode>),
}

enum InternalASTNode {
    Prefix(NumberSourceId, ASTNode),
    Infix(ASTNode, NumberSourceId, ASTNode),
    Postfix(ASTNode, NumberSourceId),
    Function(NumberSourceId, Vec<ASTNode>),
    Variable(String),
    GraphInput(NumberGraphInputId),
}

impl InternalASTNode {
    fn target(&self, variables: &[VariableDefinitions]) -> NumberTarget {
        match self {
            InternalASTNode::Prefix(id, _) => NumberTarget::Source(*id),
            InternalASTNode::Infix(_, id, _) => NumberTarget::Source(*id),
            InternalASTNode::Postfix(_, id) => NumberTarget::Source(*id),
            InternalASTNode::Function(id, _) => NumberTarget::Source(*id),
            InternalASTNode::Variable(name) => {
                let self_index = variables.iter().position(|v| v.name == *name).unwrap();
                let value = &variables[self_index].value;
                let variables_up_until_self = &variables[0..self_index];
                value.target(variables_up_until_self)
            }
            InternalASTNode::GraphInput(giid) => NumberTarget::GraphInput(*giid),
        }
    }
}

struct VariableDefinitions {
    name: String,
    value: InternalASTNode,
}

pub(super) struct LexicalLayout {
    variable_definitions: Vec<VariableDefinitions>,
    final_expression: ASTNode,
}

impl LexicalLayout {
    pub(super) fn generate(topo: &NumberGraphTopology) -> LexicalLayout {
        // TODO:
        // - assume one output for now
        // - start from the output, recursively creating nested AST nodes
        //   for each number source and its inputs
        // - whenever a number source is visited that has multiple outputs,
        //   create a variable assignment for that number source
        //   and give it a formulaic name (e.g. a, b, c, ...)
        let outputs = topo.graph_outputs();
        assert_eq!(outputs.len(), 1);
        let output = &topo.graph_outputs()[0];

        let mut variable_assignments: Vec<VariableDefinitions> = Vec::new();

        fn visit_target(
            target: NumberTarget,
            variable_assignments: &mut Vec<VariableDefinitions>,
            topo: &NumberGraphTopology,
        ) -> InternalASTNode {
            let nsid = match target {
                NumberTarget::Source(nsid) => nsid,
                NumberTarget::GraphInput(giid) => return InternalASTNode::GraphInput(giid),
            };

            if let Some(existing_variable) = variable_assignments
                .iter()
                .find(|va| va.value.target(&variable_assignments) == target)
            {
                return InternalASTNode::Variable(existing_variable.name.clone());
            }

            if topo.number_target_destinations(target).count() >= 2 {
                let value = visit_target(target, variable_assignments, topo);
                let new_variable_name = format!("x{}", variable_assignments.len());
                variable_assignments.push(VariableDefinitions {
                    name: new_variable_name.clone(),
                    value,
                });
                return InternalASTNode::Variable(new_variable_name);
            }

            // TODO: let number source uis define whether they are infix, postfix, etc
            // assuming all function calls for now

            let arguments = topo
                .number_source(nsid)
                .unwrap()
                .number_inputs()
                .iter()
                .map(|niid| match topo.number_input(*niid).unwrap().target() {
                    Some(target) => {
                        let node = visit_target(target, variable_assignments, topo);
                        ASTNode::Full(Box::new(node))
                    }
                    None => ASTNode::Empty,
                })
                .collect();

            InternalASTNode::Function(nsid, arguments)
        }

        let final_expression = match output.target() {
            Some(target) => {
                let node = visit_target(target, &mut variable_assignments, topo);
                ASTNode::Full(Box::new(node))
            }
            None => ASTNode::Empty,
        };

        LexicalLayout {
            variable_definitions: variable_assignments,
            final_expression,
        }
    }

    pub(super) fn show(&self, ui: &mut egui::Ui, result_label: &str, ctx: &NumberGraphUiContext) {
        ui.vertical(|ui| {
            for var_assn in &self.variable_definitions {
                ui.horizontal(|ui| {
                    // TODO: make this and other text pretty
                    ui.label(format!("{} = ", var_assn.name));
                    self.show_internal_node(ui, &var_assn.value, ctx);
                    ui.label(",");
                });
            }
            if !self.variable_definitions.is_empty() {
                ui.separator();
            }
            ui.horizontal(|ui| {
                ui.label(format!("{} = ", result_label));
                self.show_ast_node(ui, &self.final_expression, ctx);
                ui.label(".");
            });
        });
    }

    fn show_ast_node(&self, ui: &mut egui::Ui, node: &ASTNode, ctx: &NumberGraphUiContext) {
        match node {
            ASTNode::Empty => {
                ui.label("???");
            }
            ASTNode::Full(n) => {
                self.show_internal_node(ui, &*n, ctx);
            }
        };
    }

    fn show_internal_node(
        &self,
        ui: &mut egui::Ui,
        node: &InternalASTNode,
        ctx: &NumberGraphUiContext,
    ) {
        let styled_text = |ui: &mut egui::Ui, s: String| {
            let text = egui::RichText::new(s).code().color(egui::Color32::WHITE);
            ui.add(egui::Label::new(text));
        };

        match node {
            InternalASTNode::Prefix(nsid, expr) => {
                styled_text(ui, self.number_source_token(*nsid, ctx));
                self.show_ast_node(ui, expr, ctx);
            }
            InternalASTNode::Infix(expr1, nsid, expr2) => {
                self.show_ast_node(ui, expr1, ctx);
                styled_text(ui, self.number_source_token(*nsid, ctx));
                self.show_ast_node(ui, expr2, ctx);
            }
            InternalASTNode::Postfix(expr, nsid) => {
                self.show_ast_node(ui, expr, ctx);
                styled_text(ui, self.number_source_token(*nsid, ctx));
            }
            InternalASTNode::Function(nsid, exprs) => {
                styled_text(ui, format!("{}(", self.number_source_token(*nsid, ctx)));
                if let Some((last_expr, other_exprs)) = exprs.split_last() {
                    for expr in other_exprs {
                        self.show_ast_node(ui, expr, ctx);
                        styled_text(ui, ",".to_string());
                    }
                    self.show_ast_node(ui, last_expr, ctx);
                }
                styled_text(ui, ")".to_string());
            }
            InternalASTNode::Variable(name) => {
                styled_text(ui, name.clone());
            }
            InternalASTNode::GraphInput(giid) => {
                styled_text(ui, format!("input{}", giid.value()));
            }
        };
    }

    fn number_source_token(&self, id: NumberSourceId, ctx: &NumberGraphUiContext) -> String {
        ctx.topology()
            .number_source(id)
            .unwrap()
            .instance_arc()
            .as_graph_object()
            .get_type()
            .name()
            .to_string()
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        // TODO: check whether anything was removed, update the layout somehow.
        // This might be a lot of work and should only be done conservatively
    }
}
