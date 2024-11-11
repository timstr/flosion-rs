use eframe::egui;
use hashstash::{Order, Stash, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::{
    core::{
        expression::{
            expressiongraph::ExpressionGraph, expressiongraph::ExpressionTarget,
            expressionnode::ExpressionNodeId,
        },
        objecttype::{ObjectType, WithObjectType},
    },
    objects::purefunctions::Constant,
    ui_core::{
        arguments::ParsedArguments,
        expressiongraphuicontext::OuterExpressionGraphUiContext,
        expressiongraphuistate::ExpressionGraphUiState,
        expressionobjectui::{show_expression_node_ui, ExpressionObjectUiFactory},
        factories::Factories,
        lexicallayout::{
            ast::{ASTNodeValue, InternalASTNodeValue},
            edits::remove_unreferenced_parameters,
            validation::lexical_layout_matches_expression_graph,
        },
    },
};

use crate::ui_core::{
    expressiongraphuicontext::ExpressionGraphUiContext,
    expressiongraphuistate::ExpressionNodeObjectUiStates,
    summon_widget::{SummonWidget, SummonWidgetState},
};

use super::{
    ast::{
        find_variable_definition, ASTNode, ASTPath, ASTPathBuilder, ASTRoot, InternalASTNode,
        VariableDefinition, VariableId,
    },
    cursor::{LexicalLayoutCursor, LineLocation},
    edits::{delete_from_graph_at_cursor, insert_to_graph_at_cursor},
    summon::{build_summon_widget_for_processor_expression, ExpressionSummonValue},
};

impl Default for ExpressionNodeLayout {
    fn default() -> Self {
        ExpressionNodeLayout::Function
    }
}

#[derive(Copy, Clone)]
pub enum ExpressionNodeLayout {
    Prefix,
    Infix,
    Postfix,
    Function,
}

pub(crate) struct LexicalLayoutFocus {
    cursor: LexicalLayoutCursor,
    summon_widget_state: Option<SummonWidgetState<ExpressionSummonValue>>,
}

impl LexicalLayoutFocus {
    pub(crate) fn new() -> LexicalLayoutFocus {
        LexicalLayoutFocus {
            cursor: LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning()),
            summon_widget_state: None,
        }
    }

    pub(crate) fn cursor(&self) -> &LexicalLayoutCursor {
        &self.cursor
    }

    pub(crate) fn cursor_mut(&mut self) -> &mut LexicalLayoutCursor {
        &mut self.cursor
    }

    pub(super) fn summon_widget_state(&self) -> Option<&SummonWidgetState<ExpressionSummonValue>> {
        self.summon_widget_state.as_ref()
    }

    pub(super) fn summon_widget_state_mut(
        &mut self,
    ) -> Option<&mut SummonWidgetState<ExpressionSummonValue>> {
        self.summon_widget_state.as_mut()
    }

    pub(super) fn open_summon_widget(&mut self, state: SummonWidgetState<ExpressionSummonValue>) {
        self.summon_widget_state = Some(state);
    }

    pub(super) fn close_summon_widget(&mut self) {
        self.summon_widget_state = None;
    }
}

impl Stashable for LexicalLayoutFocus {
    fn stash(&self, stasher: &mut Stasher) {
        self.cursor.stash(stasher);
        // Not stashing summon widget
    }
}

impl Unstashable for LexicalLayoutFocus {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(LexicalLayoutFocus {
            cursor: LexicalLayoutCursor::unstash(unstasher)?,
            summon_widget_state: None,
        })
    }
}

fn make_internal_node(
    expression_node_id: ExpressionNodeId,
    layout: ExpressionNodeLayout,
    arguments: Vec<ASTNode>,
) -> InternalASTNode {
    let value = match layout {
        ExpressionNodeLayout::Prefix => {
            assert_eq!(arguments.len(), 1);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Prefix(expression_node_id, args.next().unwrap())
        }
        ExpressionNodeLayout::Infix => {
            assert_eq!(arguments.len(), 2);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Infix(
                args.next().unwrap(),
                expression_node_id,
                args.next().unwrap(),
            )
        }
        ExpressionNodeLayout::Postfix => {
            assert_eq!(arguments.len(), 1);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Postfix(args.next().unwrap(), expression_node_id)
        }
        ExpressionNodeLayout::Function => {
            InternalASTNodeValue::Function(expression_node_id, arguments)
        }
    };
    InternalASTNode::new(value)
}

fn algebraic_key(key: egui::Key, modifiers: egui::Modifiers) -> Option<char> {
    match key {
        egui::Key::Minus => {
            if !modifiers.shift {
                Some('-')
            } else {
                None
            }
        }
        egui::Key::Plus => {
            if modifiers.shift {
                Some('+')
            } else {
                None
            }
        }
        egui::Key::Num0 => Some('0'),
        egui::Key::Num1 => Some('1'),
        egui::Key::Num2 => Some('2'),
        egui::Key::Num3 => Some('3'),
        egui::Key::Num4 => Some('4'),
        egui::Key::Num5 => Some('5'),
        egui::Key::Num6 => Some('6'),
        egui::Key::Num7 => Some('7'),
        egui::Key::Num8 => Some('8'),
        egui::Key::Num9 => Some('9'),
        egui::Key::A => Some('a'),
        egui::Key::B => Some('b'),
        egui::Key::C => Some('c'),
        egui::Key::D => Some('d'),
        egui::Key::E => Some('e'),
        egui::Key::F => Some('f'),
        egui::Key::G => Some('g'),
        egui::Key::H => Some('h'),
        egui::Key::I => Some('i'),
        egui::Key::J => Some('j'),
        egui::Key::K => Some('k'),
        egui::Key::L => Some('l'),
        egui::Key::M => Some('m'),
        egui::Key::N => Some('n'),
        egui::Key::O => Some('o'),
        egui::Key::P => Some('p'),
        egui::Key::Q => Some('q'),
        egui::Key::R => Some('r'),
        egui::Key::S => Some('s'),
        egui::Key::T => Some('t'),
        egui::Key::U => Some('u'),
        egui::Key::V => Some('v'),
        egui::Key::W => Some('w'),
        egui::Key::X => Some('x'),
        egui::Key::Y => Some('y'),
        egui::Key::Z => Some('z'),
        _ => None,
    }
}

pub(crate) struct LexicalLayout {
    variable_definitions: Vec<VariableDefinition>,
    final_expression: ASTNode,
}

impl LexicalLayout {
    pub(crate) fn generate(
        graph: &ExpressionGraph,
        object_ui_states: &ExpressionNodeObjectUiStates,
        ui_factory: &ExpressionObjectUiFactory,
    ) -> LexicalLayout {
        let outputs = graph.results();
        assert_eq!(outputs.len(), 1);
        let output = &graph.results()[0];

        let mut variable_assignments: Vec<VariableDefinition> = Vec::new();

        fn visit_target(
            target: ExpressionTarget,
            variable_assignments: &mut Vec<VariableDefinition>,
            graph: &ExpressionGraph,
            object_ui_states: &ExpressionNodeObjectUiStates,
            ui_factory: &ExpressionObjectUiFactory,
        ) -> ASTNode {
            let nsid = match target {
                ExpressionTarget::Node(nsid) => nsid,
                ExpressionTarget::Parameter(giid) => {
                    return ASTNode::new(ASTNodeValue::Parameter(giid))
                }
            };

            if let Some(existing_variable) = variable_assignments
                .iter()
                .find(|va| va.value().direct_target() == Some(target))
            {
                return ASTNode::new(ASTNodeValue::Variable(existing_variable.id()));
            }

            let create_new_variable = graph.inputs_connected_to(target).len() >= 2;

            let node = graph.node(nsid).unwrap();

            let mut arguments = Vec::<ASTNode>::new();

            node.foreach_input(|input, _| {
                let value = match input.target() {
                    Some(target) => visit_target(
                        target,
                        variable_assignments,
                        graph,
                        object_ui_states,
                        ui_factory,
                    ),
                    None => ASTNode::new(ASTNodeValue::Empty),
                };
                arguments.push(value);
            });

            let object_ui = ui_factory.get(node.as_graph_object().get_dynamic_type());

            let layout = object_ui.make_properties();

            let node = make_internal_node(nsid, layout, arguments);

            if create_new_variable {
                let id = VariableId::new_unique();
                let new_variable_name = format!("x{}", variable_assignments.len() + 1);
                variable_assignments.push(VariableDefinition::new(
                    id,
                    new_variable_name.clone(),
                    ASTNode::new(ASTNodeValue::Internal(Box::new(node))),
                ));
                ASTNode::new(ASTNodeValue::Variable(id))
            } else {
                ASTNode::new(ASTNodeValue::Internal(Box::new(node)))
            }
        }

        let final_expression = match output.target() {
            Some(target) => visit_target(
                target,
                &mut variable_assignments,
                graph,
                object_ui_states,
                ui_factory,
            ),
            None => ASTNode::new(ASTNodeValue::Empty),
        };

        let layout = LexicalLayout {
            variable_definitions: variable_assignments,
            final_expression,
        };

        debug_assert!(lexical_layout_matches_expression_graph(&layout, graph));

        layout
    }

    pub(super) fn variable_definitions(&self) -> &[VariableDefinition] {
        &self.variable_definitions
    }

    pub(super) fn variable_definitions_mut(&mut self) -> &mut Vec<VariableDefinition> {
        &mut self.variable_definitions
    }

    pub(super) fn final_expression(&self) -> &ASTNode {
        &self.final_expression
    }

    pub(super) fn final_expression_mut(&mut self) -> &mut ASTNode {
        &mut self.final_expression
    }

    pub(crate) fn show(
        &self,
        ui: &mut egui::Ui,
        ui_state: &mut ExpressionGraphUiState,
        expr_graph: &mut ExpressionGraph,
        ctx: &ExpressionGraphUiContext,
        outer_context: &OuterExpressionGraphUiContext,
    ) {
        debug_assert!(lexical_layout_matches_expression_graph(self, expr_graph));

        let num_variable_definitions = self.variable_definitions.len();

        ui.vertical(|ui| {
            for i in 0..num_variable_definitions {
                self.show_line(
                    ui,
                    LineLocation::VariableDefinition(i),
                    ui_state,
                    expr_graph,
                    ctx,
                    outer_context,
                );
            }
            if num_variable_definitions > 0 {
                ui.separator();
            }
            self.show_line(
                ui,
                LineLocation::FinalExpression,
                ui_state,
                expr_graph,
                ctx,
                outer_context,
            );
        });

        debug_assert!(lexical_layout_matches_expression_graph(self, expr_graph));
    }

    fn show_line(
        &self,
        ui: &mut egui::Ui,
        line: LineLocation,
        ui_state: &mut ExpressionGraphUiState,
        expr_graph: &mut ExpressionGraph,
        ctx: &ExpressionGraphUiContext,
        outer_context: &OuterExpressionGraphUiContext,
    ) {
        ui.spacing_mut().item_spacing.x = 0.0;

        ui.horizontal(|ui| {
            match line {
                LineLocation::VariableDefinition(i) => {
                    ui.add(egui::Label::new(
                        egui::RichText::new("let ")
                            .text_style(egui::TextStyle::Monospace)
                            .background_color(egui::Color32::TRANSPARENT),
                    ));

                    let defn = &self.variable_definitions[i];

                    let name_response = ui.add(egui::Label::new(
                        egui::RichText::new(defn.name())
                            .text_style(egui::TextStyle::Monospace)
                            .strong()
                            .background_color(egui::Color32::TRANSPARENT),
                    ));

                    defn.name_rect.set(name_response.rect);
                }
                LineLocation::FinalExpression => {
                    let outputs = expr_graph.results();
                    assert_eq!(outputs.len(), 1);
                    let output_id = outputs[0].id();

                    ui.add(egui::Label::new(
                        egui::RichText::new(outer_context.result_name(output_id))
                            .text_style(egui::TextStyle::Monospace)
                            .background_color(egui::Color32::TRANSPARENT),
                    ));
                }
            }
            ui.add(egui::Label::new(
                egui::RichText::new(" = ")
                    .text_style(egui::TextStyle::Monospace)
                    .background_color(egui::Color32::TRANSPARENT),
            ));

            let (node, ast_root) = match line {
                LineLocation::VariableDefinition(i) => {
                    let defn = &self.variable_definitions[i];
                    (defn.value(), ASTRoot::VariableDefinition(defn.id()))
                }
                LineLocation::FinalExpression => (&self.final_expression, ASTRoot::FinalExpression),
            };

            Self::show_child_ast_node(
                ui,
                node,
                ui_state,
                expr_graph,
                ctx,
                ASTPathBuilder::Root(ast_root),
                outer_context,
                &self.variable_definitions,
            );

            match line {
                LineLocation::VariableDefinition(_) => ui.label(","),
                LineLocation::FinalExpression => ui.label("."),
            };
        });
    }

    fn show_child_ast_node(
        ui: &mut egui::Ui,
        node: &ASTNode,
        ui_state: &mut ExpressionGraphUiState,
        expr_graph: &mut ExpressionGraph,
        ctx: &ExpressionGraphUiContext,
        path: ASTPathBuilder,
        outer_context: &OuterExpressionGraphUiContext,
        variable_definitions: &[VariableDefinition],
    ) {
        Self::highlight_on_hover(ui, |ui| {
            let rect;

            match node.value() {
                ASTNodeValue::Empty => {
                    rect = ui.label("???").rect;
                }
                ASTNodeValue::Internal(n) => {
                    rect = Self::show_internal_node(
                        ui,
                        n,
                        ui_state,
                        expr_graph,
                        ctx,
                        path,
                        outer_context,
                        variable_definitions,
                    )
                    .rect
                }
                ASTNodeValue::Variable(id) => {
                    rect = ui
                        .add(egui::Label::new(
                            egui::RichText::new(
                                find_variable_definition(*id, variable_definitions)
                                    .unwrap()
                                    .name(),
                            )
                            .code()
                            .color(egui::Color32::WHITE),
                        ))
                        .rect;
                }
                ASTNodeValue::Parameter(giid) => {
                    let name = outer_context.parameter_name(*giid);
                    rect = ui
                        .add(egui::Label::new(
                            egui::RichText::new(name).code().color(egui::Color32::WHITE),
                        ))
                        .rect;
                }
            };
            node.set_rect(rect);
        });
    }

    fn show_internal_node(
        ui: &mut egui::Ui,
        node: &InternalASTNode,
        ui_state: &mut ExpressionGraphUiState,
        expr_graph: &mut ExpressionGraph,
        ctx: &ExpressionGraphUiContext,
        path: ASTPathBuilder,
        outer_context: &OuterExpressionGraphUiContext,
        variable_definitions: &[VariableDefinition],
    ) -> egui::Response {
        let styled_text = |ui: &mut egui::Ui, s: String| -> egui::Response {
            let text = egui::RichText::new(s).code().color(egui::Color32::WHITE);
            ui.add(egui::Label::new(text))
        };

        // TODO: clean this up also

        let ir = ui.horizontal_centered(|ui| {
            let own_rect;

            match &node.value() {
                InternalASTNodeValue::Prefix(nsid, expr) => {
                    own_rect = Self::highlight_on_hover(ui, |ui| {
                        Self::show_expression_node_ui(ui, *nsid, ui_state, expr_graph, ctx)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        ui_state,
                        expr_graph,
                        ctx,
                        path.push(node, 0),
                        outer_context,
                        variable_definitions,
                    );
                }
                InternalASTNodeValue::Infix(expr1, nsid, expr2) => {
                    Self::show_child_ast_node(
                        ui,
                        expr1,
                        ui_state,
                        expr_graph,
                        ctx,
                        path.push(node, 0),
                        outer_context,
                        variable_definitions,
                    );
                    own_rect = Self::highlight_on_hover(ui, |ui| {
                        Self::show_expression_node_ui(ui, *nsid, ui_state, expr_graph, ctx)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr2,
                        ui_state,
                        expr_graph,
                        ctx,
                        path.push(node, 1),
                        outer_context,
                        variable_definitions,
                    );
                }
                InternalASTNodeValue::Postfix(expr, nsid) => {
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        ui_state,
                        expr_graph,
                        ctx,
                        path.push(node, 0),
                        outer_context,
                        variable_definitions,
                    );
                    own_rect = Self::highlight_on_hover(ui, |ui| {
                        Self::show_expression_node_ui(ui, *nsid, ui_state, expr_graph, ctx)
                    });
                }
                InternalASTNodeValue::Function(nsid, exprs) => {
                    if exprs.is_empty() {
                        own_rect = Self::highlight_on_hover(ui, |ui| {
                            Self::show_expression_node_ui(ui, *nsid, ui_state, expr_graph, ctx)
                        })
                    } else {
                        let frame = egui::Frame::default()
                            .inner_margin(2.0)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(32)));
                        let r = frame.show(ui, |ui| {
                            let r = Self::highlight_on_hover(ui, |ui| {
                                Self::show_expression_node_ui(ui, *nsid, ui_state, expr_graph, ctx)
                            });
                            styled_text(ui, "(".to_string());
                            if let Some((last_expr, other_exprs)) = exprs.split_last() {
                                for (i, expr) in other_exprs.iter().enumerate() {
                                    Self::show_child_ast_node(
                                        ui,
                                        expr,
                                        ui_state,
                                        expr_graph,
                                        ctx,
                                        path.push(node, i),
                                        outer_context,
                                        variable_definitions,
                                    );
                                    styled_text(ui, ",".to_string());
                                }
                                Self::show_child_ast_node(
                                    ui,
                                    last_expr,
                                    ui_state,
                                    expr_graph,
                                    ctx,
                                    path.push(node, other_exprs.len()),
                                    outer_context,
                                    variable_definitions,
                                );
                            }
                            styled_text(ui, ")".to_string());
                            r
                        });

                        own_rect = r.inner;
                    }
                }
            };

            node.set_self_rect(own_rect);
        });

        ir.response
    }

    fn show_expression_node_ui(
        ui: &mut egui::Ui,
        id: ExpressionNodeId,
        ui_state: &mut ExpressionGraphUiState,
        expr_graph: &mut ExpressionGraph,
        ctx: &ExpressionGraphUiContext,
    ) -> egui::Rect {
        let graph_object = expr_graph.node_mut(id).unwrap().as_graph_object_mut();

        ui.horizontal_centered(|ui| {
            // Huh?
            show_expression_node_ui(ctx.ui_factory(), graph_object, ui_state, ui, ctx);
        })
        .response
        .rect
    }

    fn highlight_on_hover<R, F: FnOnce(&mut egui::Ui) -> R>(
        ui: &mut egui::Ui,
        add_contents: F,
    ) -> R {
        // TODO: get size of contents, highlight
        add_contents(ui)
    }

    pub(crate) fn handle_keypress(
        &mut self,
        ui: &mut egui::Ui,
        focus: &mut LexicalLayoutFocus,
        expr_graph: &mut ExpressionGraph,
        factories: &Factories,
        stash: &Stash,
        object_ui_states: &mut ExpressionNodeObjectUiStates,
        outer_context: &mut OuterExpressionGraphUiContext,
    ) {
        debug_assert!(lexical_layout_matches_expression_graph(self, expr_graph));

        self.handle_summon_widget(
            ui,
            focus,
            expr_graph,
            factories,
            stash,
            object_ui_states,
            outer_context,
        );

        if focus.summon_widget_state().is_none() {
            let cursor = focus.cursor_mut();
            let (pressed_left, pressed_right, pressed_up, pressed_down) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                )
            });

            if pressed_left {
                cursor.go_left(self);
                outer_context.request_snapshot();
            }
            if pressed_right {
                cursor.go_right(self);
                outer_context.request_snapshot();
            }
            if pressed_up {
                cursor.go_up(self);
                outer_context.request_snapshot();
            }
            if pressed_down {
                cursor.go_down(self);
                outer_context.request_snapshot();
            }

            let (pressed_delete, pressed_enter, pressed_shift_enter) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter),
                )
            });

            if pressed_delete {
                delete_from_graph_at_cursor(self, cursor, expr_graph, stash, factories);
                remove_unreferenced_parameters(self, outer_context, expr_graph);
                outer_context.request_snapshot();
            }

            if pressed_enter || pressed_shift_enter {
                let new_var_index = match cursor.line() {
                    LineLocation::VariableDefinition(i) => {
                        if pressed_shift_enter {
                            i
                        } else {
                            i + 1
                        }
                    }
                    LineLocation::FinalExpression => self.variable_definitions().len(),
                };
                let new_var_id = VariableId::new_unique();

                // Scan for autogenerated variable names with numeric
                // suffixes and find the next highest one
                let mut next_highest_index = 1;
                for var_def in &self.variable_definitions {
                    let Some(after_x) = var_def.name.strip_prefix("x") else {
                        continue;
                    };
                    if let Ok(n) = after_x.parse::<usize>() {
                        next_highest_index = next_highest_index.max(n + 1);
                    }
                }

                let new_var_name = format!("x{}", next_highest_index);
                self.variable_definitions.insert(
                    new_var_index,
                    VariableDefinition::new(
                        new_var_id,
                        new_var_name,
                        ASTNode::new(ASTNodeValue::Empty),
                    ),
                );
                *cursor = LexicalLayoutCursor::AtVariableName(new_var_index);
                outer_context.request_snapshot();
            }
        }

        debug_assert!(lexical_layout_matches_expression_graph(self, expr_graph));
    }

    fn handle_summon_widget(
        &mut self,
        ui: &mut egui::Ui,
        focus: &mut LexicalLayoutFocus,
        expr_graph: &mut ExpressionGraph,
        factories: &Factories,
        stash: &Stash,
        object_ui_states: &mut ExpressionNodeObjectUiStates,
        outer_context: &mut OuterExpressionGraphUiContext,
    ) {
        if focus.cursor().get_node(self).is_none() {
            return;
        }

        // Check for space/tab presses
        let pressed_space_or_tab = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
        });

        // Check for typing
        let algebraic_keys_pressed = ui.input_mut(|input| {
            let mut out_chars = Vec::new();
            input.events = input
                .events
                .iter()
                .filter(|e| {
                    if let egui::Event::Key {
                        key,
                        pressed,
                        repeat: _,
                        modifiers,
                        physical_key: _,
                    } = e
                    {
                        if *pressed && modifiers.is_none() {
                            if let Some(ch) = algebraic_key(*key, *modifiers) {
                                out_chars.push(ch);
                                return false;
                            }
                        }
                    }
                    true
                })
                .cloned()
                .collect();
            out_chars
        });

        // open summon widget when space/tab is pressed or something was typed
        if focus.summon_widget_state().is_none() {
            if pressed_space_or_tab || !algebraic_keys_pressed.is_empty() {
                if let Some(node_at_cursor) = focus.cursor().get_node(self) {
                    let mut widget_state = match outer_context {
                        OuterExpressionGraphUiContext::ProcessorExpression(sni_ctx) => {
                            build_summon_widget_for_processor_expression(
                                node_at_cursor.rect().center_bottom(),
                                factories.expression_uis(),
                                sni_ctx,
                                focus.cursor().get_variables_in_scope(self),
                            )
                        }
                    };
                    let s = String::from_iter(algebraic_keys_pressed);
                    widget_state.set_text(s);

                    focus.open_summon_widget(widget_state);
                }
            }
        }

        // Show and interact with the summon widget
        if let Some(summon_widget_state) = focus.summon_widget_state_mut() {
            let summon_widget = SummonWidget::new(summon_widget_state);
            ui.add(summon_widget);

            if summon_widget_state.was_cancelled() {
                focus.close_summon_widget();
            }
        }

        let Some(summon_widget_state) = focus.summon_widget_state_mut() else {
            return;
        };

        // If something was chosen, add it to the expression graph
        // and the layout
        if let Some(choice) = summon_widget_state.final_choice() {
            let (summon_value, arguments) = choice;

            debug_assert!(lexical_layout_matches_expression_graph(self, expr_graph));

            let (new_node, layout) = match summon_value {
                ExpressionSummonValue::ExpressionNodeType(ns_type) => self
                    .create_new_expression_node_from_type(
                        ns_type,
                        arguments,
                        factories,
                        object_ui_states,
                        expr_graph,
                    )
                    .unwrap(),
                ExpressionSummonValue::ParameterTarget(target) => {
                    let node;
                    {
                        let outer_context = match outer_context {
                            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => ctx,
                        };
                        let giid =
                            if let Some(giid) = outer_context.find_graph_id_for_target(target) {
                                giid
                            } else {
                                let giid = outer_context.connect_to_target(expr_graph, target);
                                giid
                            };
                        node = ASTNode::new(ASTNodeValue::Parameter(giid));
                    }
                    (node, ExpressionNodeLayout::Function)
                }
                ExpressionSummonValue::Variable(variable_id) => (
                    ASTNode::new(ASTNodeValue::Variable(variable_id)),
                    ExpressionNodeLayout::Function,
                ),
                ExpressionSummonValue::Constant(constant_value) => {
                    let (node, layout) = self
                        .create_new_expression_node_from_type(
                            Constant::TYPE,
                            arguments.add_or_replace(&Constant::ARG_VALUE, constant_value as f64),
                            factories,
                            object_ui_states,
                            expr_graph,
                        )
                        .unwrap();
                    (node, layout)
                }
            };
            let num_children = new_node.num_children();
            insert_to_graph_at_cursor(
                self,
                focus.cursor_mut(),
                new_node,
                expr_graph,
                stash,
                factories,
            );
            remove_unreferenced_parameters(self, outer_context, expr_graph);

            debug_assert!(lexical_layout_matches_expression_graph(self, expr_graph));

            let cursor_path = focus.cursor_mut().path_mut().unwrap();
            match layout {
                ExpressionNodeLayout::Prefix => cursor_path.go_into(0),
                ExpressionNodeLayout::Infix => cursor_path.go_into(0),
                ExpressionNodeLayout::Postfix => cursor_path.go_into(0),
                ExpressionNodeLayout::Function => {
                    if num_children > 0 {
                        cursor_path.go_into(0);
                    }
                }
            }
            focus.close_summon_widget();

            outer_context.request_snapshot();
        }
    }

    fn create_new_expression_node_from_type(
        &self,
        ns_type: ObjectType,
        arguments: ParsedArguments,
        factories: &Factories,
        object_ui_states: &mut ExpressionNodeObjectUiStates,
        expr_graph: &mut ExpressionGraph,
    ) -> Result<(ASTNode, ExpressionNodeLayout), String> {
        let new_object = factories
            .expression_objects()
            .create(ns_type.name(), &arguments);

        let object_ui = factories
            .expression_uis()
            .get(new_object.get_dynamic_type());

        let new_ui_state = object_ui
            .make_ui_state(&*new_object, arguments)
            .map_err(|e| format!("Failed to create ui state: {:?}", e))?;

        let layout = object_ui.make_properties();

        let new_node = new_object.into_boxed_expression_node().unwrap();
        let new_node_id = new_node.id();

        let num_inputs = new_node.input_locations().len();

        expr_graph.add_expression_node(new_node);

        let child_nodes: Vec<ASTNode> = (0..num_inputs)
            .map(|_| ASTNode::new(ASTNodeValue::Empty))
            .collect();

        let internal_node = make_internal_node(new_node_id, layout, child_nodes);
        let node = ASTNode::new(ASTNodeValue::Internal(Box::new(internal_node)));

        object_ui_states.set_object_data(new_node_id.into(), new_ui_state);

        Ok((node, layout))
    }

    pub(super) fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(&self, mut f: F) {
        for vardef in &self.variable_definitions {
            vardef.value().visit(
                ASTPathBuilder::Root(ASTRoot::VariableDefinition(vardef.id())),
                &mut f,
            );
        }
        self.final_expression
            .visit(ASTPathBuilder::Root(ASTRoot::FinalExpression), &mut f);
    }

    pub(super) fn visit_mut<F: FnMut(&mut ASTNode, ASTPathBuilder)>(&mut self, mut f: F) {
        for vardef in &mut self.variable_definitions {
            let VariableDefinition {
                id,
                name: _,
                name_rect: _,
                value,
            } = vardef;
            value.visit_mut(
                ASTPathBuilder::Root(ASTRoot::VariableDefinition(*id)),
                &mut f,
            );
        }
        self.final_expression
            .visit_mut(ASTPathBuilder::Root(ASTRoot::FinalExpression), &mut f);
    }

    pub(crate) fn cleanup(&mut self, graph: &ExpressionGraph) {
        fn visitor(
            node: &mut ASTNode,
            expected_target: Option<ExpressionTarget>,
            variable_definitions: &[VariableDefinition],
            graph: &ExpressionGraph,
        ) {
            let actual_target = node.indirect_target(variable_definitions);
            if expected_target == actual_target {
                // TODO: if the node is a reference to a variable,
                // note the expected target and use it to visit
                // the variable definition later

                if let Some(internal_node) = node.as_internal_node_mut() {
                    let nsid = internal_node.expression_node_id();
                    let expected_inputs = graph.node(nsid).unwrap().input_locations();
                    let expected_targets: Vec<Option<ExpressionTarget>> = expected_inputs
                        .iter()
                        .map(|niid| graph.input_target(*niid).unwrap())
                        .collect();

                    if internal_node.num_children() != expected_inputs.len() {
                        if let InternalASTNodeValue::Function(_, cs) = internal_node.value_mut() {
                            // see notes below
                            todo!("Allocate new AST nodes for function arguments")
                        } else {
                            panic!("An expression nodes modified its inputs and its ui doesn't support that");
                        }
                    }
                    match internal_node.value_mut() {
                        InternalASTNodeValue::Prefix(_, c) => {
                            visitor(c, expected_targets[0], variable_definitions, graph)
                        }
                        InternalASTNodeValue::Infix(c1, _, c2) => {
                            visitor(c1, expected_targets[0], variable_definitions, graph);
                            visitor(c2, expected_targets[1], variable_definitions, graph);
                        }
                        InternalASTNodeValue::Postfix(c, _) => {
                            visitor(c, expected_targets[0], variable_definitions, graph)
                        }
                        InternalASTNodeValue::Function(_, cs) => {
                            for (c, exp_tgt) in cs.iter_mut().zip(expected_targets) {
                                visitor(c, exp_tgt, variable_definitions, graph)
                            }
                        }
                    }
                }
            } else {
                // actual node target doesn't match
                match expected_target {
                    Some(ExpressionTarget::Parameter(giid)) => {
                        *node = ASTNode::new(ASTNodeValue::Parameter(giid))
                    }
                    Some(ExpressionTarget::Node(nsid)) => {
                        // TODO:
                        // - if an existing (direct) variable definition exists for the source,
                        //   create a reference to that variable
                        // - otherwise, if the the expression node is already referenced by
                        //   some other part of the lexical layout, extract a new variable definition
                        //   and replace both places with a reference to it
                        // - otherwise, recursively create a new AST node and place it here
                        todo!("Allocate new ast nodes")
                    }
                    None => {
                        *node = ASTNode::new(ASTNodeValue::Empty);
                    }
                }
            }
        }

        let graph_outputs = graph.results();
        assert_eq!(graph_outputs.len(), 1);
        let graph_output = &graph_outputs[0];

        visitor(
            &mut self.final_expression,
            graph_output.target(),
            &self.variable_definitions,
            graph,
        );

        // TODO: after having gathered expected targets for variable definitions,
        // visit those to confirm that they match

        // TODO: create variable definitions for any unreferenced expression nodes
    }
}

impl Stashable for LexicalLayout {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_objects_slice(&self.variable_definitions, Order::Ordered);
        stasher.object(&self.final_expression);
    }
}

impl Unstashable for LexicalLayout {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let variable_definitions = unstasher.array_of_objects_vec()?;
        let final_expression = unstasher.object()?;
        Ok(LexicalLayout {
            variable_definitions,
            final_expression,
        })
    }
}
