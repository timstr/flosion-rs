use eframe::egui;

use crate::core::{
    number::{
        numbergraph::NumberGraphInputId, numbergraphdata::NumberTarget,
        numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId,
    },
    uniqueid::UniqueId,
};

use super::{
    numbergraphuicontext::NumberGraphUiContext, numbergraphuistate::NumberGraphUiState,
    soundnumberinputui::SpatialGraphInputReference,
};

// TODO: this cursor thing is a mess because too many things are trying to describe the same thing.
// If there was simply a single unambiguous integer representation for these paths (which n.b. are
// purely internal to this file!) then this would be WAY simpler.
// Cursors and paths are only intended for editing and maneouvering through the AST, so they should
// be implemented cleanly for that purpose.
// Note also that including "just before" and "just after" doubles or triples the number of visitable
// locations (e.g. the number of keypresses needed to skip through an expression) but the control it
// offers could just as easily be obtained during actual edit actions, e.g. do XYZ to overwrite,
// do shift+XYZ to insert after, do alt+XYZ to insert after. Or use arrow keys?

#[derive(Clone)]
pub(super) struct ASTPath {
    steps: Vec<usize>,
}

impl ASTPath {
    fn new(steps: Vec<usize>) -> ASTPath {
        ASTPath { steps }
    }

    fn go_left(&mut self, tree: &InternalASTNode) {
        let Some(last_step) = self.steps.pop() else {
            return;
        };
        if last_step > 0 {
            self.steps.push(last_step - 1);
            loop {
                let node = tree.get_along_path(&self.steps);
                let num_children = node.and_then(|n| Some(n.num_children())).unwrap_or(0);
                if num_children > 0 {
                    self.steps.push(num_children - 1);
                } else {
                    break;
                }
            }
        }
    }

    fn go_right(&mut self, tree: &InternalASTNode) {
        if let Some(node) = tree.get_along_path(&self.steps) {
            if node.num_children() > 0 {
                self.steps.push(0);
                return;
            }
        }
        loop {
            let Some(last_step) = self.steps.pop() else {
                break;
            };
            let parent = tree.get_along_path(&self.steps).unwrap();
            let num_siblings = parent.num_children();
            let next_step = last_step + 1;
            if next_step < num_siblings {
                self.steps.push(next_step);
                return;
            }
        }
        loop {
            let Some(node) = tree.get_along_path(&self.steps) else {
                return;
            };
            let num_children = node.num_children();
            if num_children > 0 {
                self.steps.push(num_children - 1);
            } else {
                return;
            }
        }
    }

    fn clear(&mut self) {
        self.steps.clear();
    }
}

#[derive(Clone, Copy)]
enum ASTPathBuilder<'a> {
    Root,
    ChildOf(&'a ASTPathBuilder<'a>, usize),
}

impl<'a> ASTPathBuilder<'a> {
    fn push(&'a self, index: usize) -> ASTPathBuilder<'a> {
        ASTPathBuilder::ChildOf(self, index)
    }

    fn build(&self) -> ASTPath {
        fn helper(builder: &ASTPathBuilder, vec: &mut Vec<usize>) {
            if let ASTPathBuilder::ChildOf(parent, index) = builder {
                helper(parent, vec);
                vec.push(*index);
            }
        }

        let mut steps = Vec::new();
        helper(self, &mut steps);
        ASTPath { steps }
    }

    fn matches_path(&self, path: &ASTPath) -> bool {
        fn helper(builder: &ASTPathBuilder, steps: &[usize]) -> bool {
            match builder {
                ASTPathBuilder::Root => steps.is_empty(),
                ASTPathBuilder::ChildOf(parent, i) => {
                    let Some((last_step, other_steps)) = steps.split_last() else {
                        return false;
                    };
                    if last_step != i {
                        return false;
                    }
                    helper(parent, other_steps)
                }
            }
        }

        helper(self, &path.steps)
    }
}

enum ASTNodeValue {
    Empty,
    Internal(Box<InternalASTNode>),
    Variable(String),
    GraphInput(NumberGraphInputId),
}

struct ASTNode {
    value: ASTNodeValue,
    rect: egui::Rect,
}

impl ASTNode {
    fn new(value: ASTNodeValue) -> ASTNode {
        ASTNode {
            value,
            rect: egui::Rect::NOTHING,
        }
    }

    fn target(&self, variables: &[VariableDefinitions]) -> Option<NumberTarget> {
        match &self.value {
            ASTNodeValue::Empty => None,
            ASTNodeValue::Internal(node) => Some(node.target(variables)),
            ASTNodeValue::Variable(_) => None,
            ASTNodeValue::GraphInput(_) => None,
        }
    }

    fn internal_node(&self) -> Option<&InternalASTNode> {
        match &self.value {
            ASTNodeValue::Internal(n) => Some(&*n),
            _ => None,
        }
    }

    fn is_over(&self, p: egui::Pos2) -> bool {
        self.rect.contains(p)
    }

    fn is_directly_over(&self, p: egui::Pos2) -> bool {
        if !self.is_over(p) {
            return false;
        }
        if let ASTNodeValue::Internal(n) = &self.value {
            !(n.over_self(p) || n.over_children(p))
        } else {
            false
        }
    }

    fn count_graph_inputs(&self) -> usize {
        match &self.value {
            ASTNodeValue::Internal(n) => n.count_graph_inputs(),
            ASTNodeValue::GraphInput(_) => 1,
            _ => 0,
        }
    }
}

enum InternalASTNodeValue {
    Prefix(NumberSourceId, ASTNode),
    Infix(ASTNode, NumberSourceId, ASTNode),
    Postfix(ASTNode, NumberSourceId),
    Function(NumberSourceId, Vec<ASTNode>),
}

struct InternalASTNode {
    value: InternalASTNodeValue,
    self_rect: egui::Rect,
}

impl InternalASTNode {
    fn new(value: InternalASTNodeValue) -> InternalASTNode {
        InternalASTNode {
            value,
            self_rect: egui::Rect::NOTHING,
        }
    }

    fn target(&self, variables: &[VariableDefinitions]) -> NumberTarget {
        match &self.value {
            InternalASTNodeValue::Prefix(id, _) => NumberTarget::Source(*id),
            InternalASTNodeValue::Infix(_, id, _) => NumberTarget::Source(*id),
            InternalASTNodeValue::Postfix(_, id) => NumberTarget::Source(*id),
            InternalASTNodeValue::Function(id, _) => NumberTarget::Source(*id),
        }
    }

    fn num_children(&self) -> usize {
        match &self.value {
            InternalASTNodeValue::Prefix(_, _) => 1,
            InternalASTNodeValue::Infix(_, _, _) => 2,
            InternalASTNodeValue::Postfix(_, _) => 1,
            InternalASTNodeValue::Function(_, c) => c.len(),
        }
    }

    fn over_self(&self, p: egui::Pos2) -> bool {
        self.self_rect.contains(p)
    }

    fn over_children(&self, p: egui::Pos2) -> bool {
        match &self.value {
            InternalASTNodeValue::Prefix(_, c) => c.is_over(p),
            InternalASTNodeValue::Infix(c1, _, c2) => c1.is_over(p) || c2.is_over(p),
            InternalASTNodeValue::Postfix(c, _) => c.is_over(p),
            InternalASTNodeValue::Function(_, cs) => cs.iter().any(|c| c.is_over(p)),
        }
    }

    fn count_graph_inputs(&self) -> usize {
        match &self.value {
            InternalASTNodeValue::Prefix(_, c) => c.count_graph_inputs(),
            InternalASTNodeValue::Infix(c1, _, c2) => {
                c1.count_graph_inputs() + c2.count_graph_inputs()
            }
            InternalASTNodeValue::Postfix(c, _) => c.count_graph_inputs(),
            InternalASTNodeValue::Function(_, cs) => {
                cs.iter().map(|c| c.count_graph_inputs()).sum()
            }
        }
    }

    fn get_along_path(&self, path: &[usize]) -> Option<&InternalASTNode> {
        let Some((next_step, rest_of_path)) = path.split_first() else {
            return Some(self);
        };
        let child_node = match (next_step, &self.value) {
            (0, InternalASTNodeValue::Prefix(_, c)) => c,
            (0, InternalASTNodeValue::Infix(c, _, _)) => c,
            (1, InternalASTNodeValue::Infix(_, _, c)) => c,
            (0, InternalASTNodeValue::Postfix(c, _)) => c,
            (i, InternalASTNodeValue::Function(_, cs)) => &cs[*i],
            _ => panic!(),
        };
        match &child_node.value {
            ASTNodeValue::Internal(node) => node.get_along_path(rest_of_path),
            _ => {
                assert!(rest_of_path.is_empty());
                None
            }
        }
    }
}

pub(super) struct Cursor {
    line: usize,
    path: ASTPath,
}

struct VariableDefinitions {
    name: String,
    value: ASTNode,
}

pub(super) struct LexicalLayout {
    variable_definitions: Vec<VariableDefinitions>,
    final_expression: ASTNode,
}

impl LexicalLayout {
    pub(super) fn generate(topo: &NumberGraphTopology) -> LexicalLayout {
        let outputs = topo.graph_outputs();
        assert_eq!(outputs.len(), 1);
        let output = &topo.graph_outputs()[0];

        let mut variable_assignments: Vec<VariableDefinitions> = Vec::new();

        fn visit_target(
            target: NumberTarget,
            variable_assignments: &mut Vec<VariableDefinitions>,
            topo: &NumberGraphTopology,
        ) -> ASTNode {
            let nsid = match target {
                NumberTarget::Source(nsid) => nsid,
                NumberTarget::GraphInput(giid) => {
                    return ASTNode::new(ASTNodeValue::GraphInput(giid))
                }
            };

            if let Some(existing_variable) = variable_assignments
                .iter()
                .find(|va| va.value.target(&variable_assignments) == Some(target))
            {
                return ASTNode::new(ASTNodeValue::Variable(existing_variable.name.clone()));
            }

            let create_new_variable = topo.number_target_destinations(target).count() >= 2;

            // TODO: let number source uis define whether they are infix, postfix, etc
            // assuming all function calls for now

            let arguments = topo
                .number_source(nsid)
                .unwrap()
                .number_inputs()
                .iter()
                .map(|niid| match topo.number_input(*niid).unwrap().target() {
                    Some(target) => visit_target(target, variable_assignments, topo),
                    None => ASTNode::new(ASTNodeValue::Empty),
                })
                .collect();

            let value = InternalASTNode::new(InternalASTNodeValue::Function(nsid, arguments));

            if create_new_variable {
                let new_variable_name = format!("x{}", variable_assignments.len());
                variable_assignments.push(VariableDefinitions {
                    name: new_variable_name.clone(),
                    value: ASTNode::new(ASTNodeValue::Internal(Box::new(value))),
                });
                ASTNode::new(ASTNodeValue::Variable(new_variable_name))
            } else {
                ASTNode::new(ASTNodeValue::Internal(Box::new(value)))
            }
        }

        let final_expression = match output.target() {
            Some(target) => visit_target(target, &mut variable_assignments, topo),
            None => ASTNode::new(ASTNodeValue::Empty),
        };

        LexicalLayout {
            variable_definitions: variable_assignments,
            final_expression,
        }
    }

    pub(super) fn show(
        &mut self,
        ui: &mut egui::Ui,
        result_label: &str,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        cursor: &mut Option<Cursor>,
    ) -> Vec<SpatialGraphInputReference> {
        let variable_definitions = &mut self.variable_definitions;
        let num_variable_definitions = variable_definitions.len();
        let final_expression = &mut self.final_expression;
        let mut graph_input_references = Vec::new();

        ui.vertical(|ui| {
            for (i, var_assn) in variable_definitions.iter_mut().enumerate() {
                let line_number = i;
                Self::show_line(
                    ui,
                    &mut var_assn.value,
                    &mut graph_input_references,
                    cursor,
                    line_number,
                    |ui, graph_input_references, cursor, node| {
                        ui.horizontal(|ui| {
                            // TODO: make this and other text pretty
                            ui.label(format!("{} = ", var_assn.name));
                            Self::show_child_ast_node(
                                ui,
                                node,
                                graph_state,
                                ctx,
                                ASTPathBuilder::Root,
                                cursor,
                                graph_input_references,
                            );
                            ui.label(",");
                        });
                    },
                );
            }
            if num_variable_definitions > 0 {
                ui.separator();
            }
            let line_number = variable_definitions.len();
            Self::show_line(
                ui,
                final_expression,
                &mut graph_input_references,
                cursor,
                line_number,
                |ui, graph_input_references, cursor, node| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} = ", result_label));
                        Self::show_child_ast_node(
                            ui,
                            node,
                            graph_state,
                            ctx,
                            ASTPathBuilder::Root,
                            cursor,
                            graph_input_references,
                        );
                        ui.label(".");
                    });
                },
            );
        });

        if let Some(cursor) = cursor.as_mut() {
            let (pressed_up, pressed_down) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                )
            });
            if pressed_up {
                cursor.line = cursor.line.saturating_sub(1);
                cursor.path.clear();
            }
            if pressed_down {
                cursor.line = (cursor.line + 1).min(variable_definitions.len());
                cursor.path.clear();
            }
        }

        graph_input_references
    }

    fn show_line<
        F: FnOnce(
            &mut egui::Ui,
            &mut Vec<SpatialGraphInputReference>,
            &mut Option<ASTPath>,
            &mut ASTNode,
        ),
    >(
        ui: &mut egui::Ui,
        node: &mut ASTNode,
        graph_input_references: &mut Vec<SpatialGraphInputReference>,
        cursor: &mut Option<Cursor>,
        line_number: usize,
        add_contents: F,
    ) {
        // TODO: share this with soundprocessorui
        let input_reference_height = 5.0;

        let num_graph_inputs = node.count_graph_inputs();
        let num_inputs_before = graph_input_references.len();
        let top_of_line = ui.cursor().top();
        ui.add_space(input_reference_height * num_graph_inputs as f32);

        let mut cursor_path = if let Some(cursor) = cursor {
            if cursor.line == line_number {
                Some(cursor.path.clone())
            } else {
                None
            }
        } else {
            None
        };

        add_contents(ui, graph_input_references, &mut cursor_path, node);

        if let Some(mut path) = cursor_path {
            let (pressed_left, pressed_right) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowLeft),
                    i.key_pressed(egui::Key::ArrowRight),
                )
            });

            if let Some(n) = node.internal_node() {
                if pressed_left {
                    path.go_left(n);
                }
                if pressed_right {
                    path.go_right(n);
                }
            }

            *cursor = Some(Cursor {
                line: line_number,
                path,
            });
        }

        let num_inputs_after = graph_input_references.len();
        debug_assert_eq!(num_inputs_before + num_graph_inputs, num_inputs_after);
        let new_references = &mut graph_input_references[num_inputs_before..];
        for (i, new_ref) in new_references.iter_mut().enumerate() {
            new_ref.location_mut().y = top_of_line + (i as f32) * input_reference_height;
        }
    }

    fn show_child_ast_node(
        ui: &mut egui::Ui,
        node: &mut ASTNode,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
        graph_input_references: &mut Vec<SpatialGraphInputReference>,
    ) {
        let hovering = ui
            .input(|i| i.pointer.hover_pos())
            .and_then(|p| Some(node.is_directly_over(p)))
            .unwrap_or(false);
        Self::with_cursor(ui, path, cursor, hovering, |ui, cursor| {
            let rect = match &mut node.value {
                ASTNodeValue::Empty => {
                    // TODO: show cursor?
                    let r = ui.label("???");
                    r.rect
                }
                ASTNodeValue::Internal(n) => {
                    let r = Self::show_internal_node(
                        ui,
                        &mut *n,
                        graph_state,
                        ctx,
                        path,
                        cursor,
                        graph_input_references,
                    );
                    r.rect
                }
                ASTNodeValue::Variable(name) => {
                    ui.add(egui::Label::new(
                        egui::RichText::new(&*name)
                            .code()
                            .color(egui::Color32::WHITE),
                    ))
                    .rect
                }
                ASTNodeValue::GraphInput(giid) => {
                    let name = format!("input{}", giid.value());
                    let r = ui
                        .add(egui::Label::new(
                            egui::RichText::new(name).code().color(egui::Color32::WHITE),
                        ))
                        .rect;
                    graph_input_references
                        .push(SpatialGraphInputReference::new(*giid, r.center_top()));
                    r
                }
            };
            node.rect = rect;
        });
    }

    fn show_internal_node(
        ui: &mut egui::Ui,
        node: &mut InternalASTNode,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
        graph_input_references: &mut Vec<SpatialGraphInputReference>,
    ) -> egui::Response {
        let styled_text = |ui: &mut egui::Ui, s: String| -> egui::Response {
            let text = egui::RichText::new(s).code().color(egui::Color32::WHITE);
            ui.add(egui::Label::new(text))
        };

        let ir = ui.horizontal_centered(|ui| {
            let hovering_over_self = ui
                .input(|i| i.pointer.hover_pos())
                .and_then(|p| Some(node.over_self(p)))
                .unwrap_or(false);
            let own_rect = match &mut node.value {
                InternalASTNodeValue::Prefix(nsid, expr) => {
                    let r = Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        graph_state,
                        ctx,
                        path.push(0),
                        cursor,
                        graph_input_references,
                    );
                    r
                }
                InternalASTNodeValue::Infix(expr1, nsid, expr2) => {
                    Self::show_child_ast_node(
                        ui,
                        expr1,
                        graph_state,
                        ctx,
                        path.push(0),
                        cursor,
                        graph_input_references,
                    );
                    let r = Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr2,
                        graph_state,
                        ctx,
                        path.push(1),
                        cursor,
                        graph_input_references,
                    );
                    r
                }
                InternalASTNodeValue::Postfix(expr, nsid) => {
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        graph_state,
                        ctx,
                        path.push(0),
                        cursor,
                        graph_input_references,
                    );
                    Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                    })
                }
                InternalASTNodeValue::Function(nsid, exprs) => {
                    if exprs.is_empty() {
                        Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                            Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                        })
                    } else {
                        let frame = egui::Frame::default()
                            .inner_margin(2.0)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(32)));
                        frame
                            .show(ui, |ui| {
                                let r = Self::with_cursor(
                                    ui,
                                    path,
                                    cursor,
                                    hovering_over_self,
                                    |ui, _| {
                                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx)
                                    },
                                );
                                styled_text(ui, "(".to_string());
                                if let Some((last_expr, other_exprs)) = exprs.split_last_mut() {
                                    for (i, expr) in other_exprs.iter_mut().enumerate() {
                                        Self::show_child_ast_node(
                                            ui,
                                            expr,
                                            graph_state,
                                            ctx,
                                            path.push(i),
                                            cursor,
                                            graph_input_references,
                                        );
                                        styled_text(ui, ",".to_string());
                                    }
                                    Self::show_child_ast_node(
                                        ui,
                                        last_expr,
                                        graph_state,
                                        ctx,
                                        path.push(other_exprs.len()),
                                        cursor,
                                        graph_input_references,
                                    );
                                }
                                styled_text(ui, ")".to_string());
                                r
                            })
                            .inner
                    }
                }
            };

            node.self_rect = own_rect;
        });

        ir.response
    }

    fn show_number_source_ui(
        ui: &mut egui::Ui,
        id: NumberSourceId,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
    ) -> egui::Rect {
        let graph_object = ctx
            .topology()
            .number_source(id)
            .unwrap()
            .instance_arc()
            .as_graph_object();
        let type_str = graph_object.get_type().name();
        let object_ui = ctx.ui_factory().get_object_ui(type_str);
        let object_state = ctx.object_ui_states().get_object_data(id);
        ui.horizontal_centered(|ui| {
            object_ui.apply(
                &graph_object,
                &mut object_state.borrow_mut(),
                graph_state,
                ui,
                ctx,
            );
        })
        .response
        .rect
    }

    fn flashing_highlight_color(ui: &mut egui::Ui) -> egui::Color32 {
        ui.ctx().request_repaint();
        let t = ui.input(|i| i.time);
        let a = (((t - t.floor()) * 2.0 * std::f64::consts::TAU).sin() * 16.0 + 64.0) as u8;
        egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, a)
    }

    fn draw_free_cursor(ui: &mut egui::Ui) {
        let (_, rect) = ui.allocate_space(egui::vec2(5.0, 20.0));
        let color = Self::flashing_highlight_color(ui);
        ui.painter()
            .rect_filled(rect, egui::Rounding::none(), color);
    }

    fn with_cursor<R, F: FnOnce(&mut egui::Ui, &mut Option<ASTPath>) -> R>(
        ui: &mut egui::Ui,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
        hovering: bool,
        add_contents: F,
    ) -> R {
        let highlight = cursor
            .as_ref()
            .and_then(|c| Some(path.matches_path(c)))
            .unwrap_or(false);
        let ret;
        {
            let color = if highlight {
                Self::flashing_highlight_color(ui)
            } else {
                // egui::Color32::TRANSPARENT
                egui::Color32::from_black_alpha(64)
            };
            let frame = egui::Frame::default()
                .inner_margin(2.0)
                .fill(color)
                .stroke(egui::Stroke::new(2.0, color));
            let r = frame.show(ui, |ui| add_contents(ui, cursor));
            ret = r.inner;

            let r = r.response.interact(egui::Sense::click_and_drag());

            if r.clicked() || r.dragged() {
                *cursor = Some(path.build());
            }
            // if r.hovered() {
            //     ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            // }

            let hover_amount = ui.ctx().animate_bool(r.id, hovering);
            if hover_amount > 0.0 {
                ui.painter().rect_stroke(
                    r.rect,
                    egui::Rounding::none(),
                    egui::Stroke::new(
                        2.0,
                        egui::Color32::from_white_alpha((hover_amount * 64.0) as u8),
                    ),
                );
            }
        }
        ret
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        // TODO: check whether anything was removed, update the layout somehow.
        // This might be a lot of work and should only be done conservatively
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_along_path() {
        let tree = InternalASTNode::new(InternalASTNodeValue::Function(
            NumberSourceId::new(1),
            vec![
                ASTNode::new(ASTNodeValue::Empty),
                ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                    InternalASTNodeValue::Function(
                        NumberSourceId::new(2),
                        vec![ASTNode::new(ASTNodeValue::Variable("foo".to_string()))],
                    ),
                )))),
                ASTNode::new(ASTNodeValue::GraphInput(NumberGraphInputId::new(11))),
                ASTNode::new(ASTNodeValue::Variable("bar".to_string())),
            ],
        ));

        let Some(tree_at_empty_path) = tree.get_along_path(&[]) else {
            panic!();
        };

        assert!(
            if let InternalASTNodeValue::Function(_, _) = tree_at_empty_path.value {
                true
            } else {
                false
            }
        );

        assert!(tree.get_along_path(&[0]).is_none());

        let Some(tree_at_path_1) = tree.get_along_path(&[1]) else {
            panic!();
        };

        assert!(
            if let InternalASTNodeValue::Function(_, _) = tree_at_path_1.value {
                true
            } else {
                false
            }
        );

        assert!(tree.get_along_path(&[2]).is_none());

        assert!(tree.get_along_path(&[3]).is_none());
    }

    #[test]
    fn test_go_left() {
        let tree = InternalASTNode::new(InternalASTNodeValue::Function(
            NumberSourceId::new(1),
            vec![
                ASTNode::new(ASTNodeValue::Empty),
                ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                    InternalASTNodeValue::Function(
                        NumberSourceId::new(2),
                        vec![ASTNode::new(ASTNodeValue::Variable("foo".to_string()))],
                    ),
                )))),
                ASTNode::new(ASTNodeValue::GraphInput(NumberGraphInputId::new(11))),
                ASTNode::new(ASTNodeValue::Variable("bar".to_string())),
            ],
        ));

        let mut path = ASTPath::new(vec![]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[]);

        let mut path = ASTPath::new(vec![0]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[]);

        let mut path = ASTPath::new(vec![1]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[0]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[]);

        let mut path = ASTPath::new(vec![1, 0]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[1]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[0]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[]);

        let mut path = ASTPath::new(vec![3]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[2]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[1, 0]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[1]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[0]);
        path.go_left(&tree);
        assert_eq!(&path.steps, &[]);
    }

    #[test]
    fn test_go_right() {
        let tree = InternalASTNode::new(InternalASTNodeValue::Function(
            NumberSourceId::new(1),
            vec![
                ASTNode::new(ASTNodeValue::Empty),
                ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                    InternalASTNodeValue::Function(
                        NumberSourceId::new(2),
                        vec![ASTNode::new(ASTNodeValue::Variable("foo".to_string()))],
                    ),
                )))),
                ASTNode::new(ASTNodeValue::GraphInput(NumberGraphInputId::new(11))),
                ASTNode::new(ASTNodeValue::Variable("bar".to_string())),
            ],
        ));

        let mut path = ASTPath::new(vec![]);
        path.go_right(&tree);
        assert_eq!(&path.steps, &[0]);
        path.go_right(&tree);
        assert_eq!(&path.steps, &[1]);
        path.go_right(&tree);
        assert_eq!(&path.steps, &[1, 0]);
        path.go_right(&tree);
        assert_eq!(&path.steps, &[2,]);
        path.go_right(&tree);
        assert_eq!(&path.steps, &[3]);
        path.go_right(&tree);
        assert_eq!(&path.steps, &[3]);
    }
}
