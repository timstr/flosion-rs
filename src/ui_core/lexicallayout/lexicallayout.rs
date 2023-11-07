use eframe::egui;
use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::{
        graph::{
            graphobject::{ObjectType, WithObjectType},
            objectfactory::ObjectFactory,
        },
        number::{
            numbergraph::NumberGraph, numbergraphdata::NumberTarget,
            numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId,
        },
        uniqueid::IdGenerator,
    },
    objects::functions::Constant,
    ui_core::{
        arguments::ParsedArguments,
        lexicallayout::{
            ast::{ASTNodeValue, InternalASTNodeValue},
            edits::remove_unreferenced_graph_inputs,
            validation::lexical_layout_matches_number_graph,
        },
        numbergraphuicontext::OuterNumberGraphUiContext,
    },
};

use crate::ui_core::{
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{AnyNumberObjectUiData, NumberGraphUiState, NumberObjectUiStates},
    summon_widget::{SummonWidget, SummonWidgetState},
    ui_factory::UiFactory,
};

use super::{
    ast::{
        find_variable_definition, ASTNode, ASTPath, ASTPathBuilder, ASTRoot, InternalASTNode,
        VariableDefinition, VariableId,
    },
    cursor::{LexicalLayoutCursor, LineLocation},
    edits::{delete_from_numbergraph_at_cursor, insert_to_numbergraph_at_cursor},
    summon::{build_summon_widget_for_sound_number_input, NumberSummonValue},
};

impl Default for NumberSourceLayout {
    fn default() -> Self {
        NumberSourceLayout::Function
    }
}

impl Serializable for NumberSourceLayout {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.u8(match self {
            NumberSourceLayout::Prefix => 1,
            NumberSourceLayout::Infix => 2,
            NumberSourceLayout::Postfix => 3,
            NumberSourceLayout::Function => 4,
        });
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(match deserializer.u8()? {
            1 => NumberSourceLayout::Prefix,
            2 => NumberSourceLayout::Infix,
            3 => NumberSourceLayout::Postfix,
            4 => NumberSourceLayout::Function,
            _ => return Err(()),
        })
    }
}

#[derive(Copy, Clone)]
pub enum NumberSourceLayout {
    Prefix,
    Infix,
    Postfix,
    Function,
}

pub(crate) struct LexicalLayoutFocus {
    cursor: LexicalLayoutCursor,
    summon_widget_state: Option<SummonWidgetState<NumberSummonValue>>,
}

impl LexicalLayoutFocus {
    pub(crate) fn new() -> LexicalLayoutFocus {
        LexicalLayoutFocus {
            cursor: LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning()),
            summon_widget_state: None,
        }
    }

    pub(super) fn cursor(&self) -> &LexicalLayoutCursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut LexicalLayoutCursor {
        &mut self.cursor
    }

    pub(super) fn summon_widget_state(&self) -> Option<&SummonWidgetState<NumberSummonValue>> {
        self.summon_widget_state.as_ref()
    }

    // TODO: return just Option<&mut SummonWidgetState...>,
    // add separate method to write the option itself
    pub(super) fn summon_widget_state_mut(
        &mut self,
    ) -> &mut Option<SummonWidgetState<NumberSummonValue>> {
        &mut self.summon_widget_state
    }

    pub(super) fn close_summon_widget(&mut self) {
        self.summon_widget_state = None;
    }
}

fn make_internal_node(
    number_source_id: NumberSourceId,
    ui_data: &AnyNumberObjectUiData,
    arguments: Vec<ASTNode>,
) -> InternalASTNode {
    let value = match ui_data.layout() {
        NumberSourceLayout::Prefix => {
            assert_eq!(arguments.len(), 1);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Prefix(number_source_id, args.next().unwrap())
        }
        NumberSourceLayout::Infix => {
            assert_eq!(arguments.len(), 2);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Infix(
                args.next().unwrap(),
                number_source_id,
                args.next().unwrap(),
            )
        }
        NumberSourceLayout::Postfix => {
            assert_eq!(arguments.len(), 1);
            let mut args = arguments.into_iter();
            InternalASTNodeValue::Postfix(args.next().unwrap(), number_source_id)
        }
        NumberSourceLayout::Function => InternalASTNodeValue::Function(number_source_id, arguments),
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
        egui::Key::PlusEquals => {
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
    variable_id_generator: IdGenerator<VariableId>,
}

impl LexicalLayout {
    pub(crate) fn generate(
        topo: &NumberGraphTopology,
        object_ui_states: &NumberObjectUiStates,
    ) -> LexicalLayout {
        let outputs = topo.graph_outputs();
        assert_eq!(outputs.len(), 1);
        let output = &topo.graph_outputs()[0];

        let mut variable_assignments: Vec<VariableDefinition> = Vec::new();

        let mut variable_id_generator = IdGenerator::<VariableId>::new();

        fn visit_target(
            target: NumberTarget,
            variable_assignments: &mut Vec<VariableDefinition>,
            topo: &NumberGraphTopology,
            object_ui_states: &NumberObjectUiStates,
            variable_id_generator: &mut IdGenerator<VariableId>,
        ) -> ASTNode {
            let nsid = match target {
                NumberTarget::Source(nsid) => nsid,
                NumberTarget::GraphInput(giid) => {
                    return ASTNode::new(ASTNodeValue::GraphInput(giid))
                }
            };

            if let Some(existing_variable) = variable_assignments
                .iter()
                .find(|va| va.value().direct_target() == Some(target))
            {
                return ASTNode::new(ASTNodeValue::Variable(existing_variable.id()));
            }

            let create_new_variable = topo.number_target_destinations(target).count() >= 2;

            let arguments: Vec<ASTNode> = topo
                .number_source(nsid)
                .unwrap()
                .number_inputs()
                .iter()
                .map(|niid| match topo.number_input(*niid).unwrap().target() {
                    Some(target) => visit_target(
                        target,
                        variable_assignments,
                        topo,
                        object_ui_states,
                        variable_id_generator,
                    ),
                    None => ASTNode::new(ASTNodeValue::Empty),
                })
                .collect();

            let node =
                make_internal_node(nsid, &*object_ui_states.get_object_data(nsid), arguments);

            if create_new_variable {
                let id = variable_id_generator.next_id();
                let new_variable_name = format!("x{}", variable_assignments.len());
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
                topo,
                object_ui_states,
                &mut variable_id_generator,
            ),
            None => ASTNode::new(ASTNodeValue::Empty),
        };

        let layout = LexicalLayout {
            variable_definitions: variable_assignments,
            final_expression,
            variable_id_generator,
        };

        debug_assert!(lexical_layout_matches_number_graph(&layout, topo));

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
        &mut self,
        ui: &mut egui::Ui,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        mut focus: Option<&mut LexicalLayoutFocus>,
        outer_context: &OuterNumberGraphUiContext,
    ) {
        debug_assert!(outer_context
            .inspect_number_graph(|g| { lexical_layout_matches_number_graph(self, g.topology()) }));

        let variable_definitions = &self.variable_definitions;
        let num_variable_definitions = variable_definitions.len();

        ui.vertical(|ui| {
            for i in 0..variable_definitions.len() {
                self.show_line(
                    ui,
                    LineLocation::VariableDefinition(i),
                    &mut focus,
                    graph_state,
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
                &mut focus,
                graph_state,
                ctx,
                outer_context,
            );
        });

        if let Some(focus) = focus {
            if let Some(summon_widget_state) = focus.summon_widget_state_mut().as_mut() {
                let summon_widget = SummonWidget::new(summon_widget_state);
                ui.add(summon_widget);

                if summon_widget_state.was_cancelled() {
                    focus.close_summon_widget();
                }
            }
        }

        debug_assert!(outer_context
            .inspect_number_graph(|g| { lexical_layout_matches_number_graph(self, g.topology()) }));
    }

    fn show_line(
        &self,
        ui: &mut egui::Ui,
        line: LineLocation,
        focus: &mut Option<&mut LexicalLayoutFocus>,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        outer_context: &OuterNumberGraphUiContext,
    ) {
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut cursor_path = if let Some(focus) = focus {
            let cursor = focus.cursor();
            if cursor.line() == line {
                cursor.path().cloned()
            } else {
                None
            }
        } else {
            None
        };

        let (node, ast_root) = match line {
            LineLocation::VariableDefinition(i) => {
                let defn = &self.variable_definitions[i];
                (
                    defn.value(),
                    ASTRoot::VariableDefinition(defn.id(), defn.name()),
                )
            }
            LineLocation::FinalExpression => (&self.final_expression, ASTRoot::FinalExpression),
        };

        ui.horizontal(|ui| {
            match line {
                LineLocation::VariableDefinition(i) => {
                    let name_in_focus = focus
                        .as_ref()
                        .and_then(|f| {
                            Some(match f.cursor() {
                                LexicalLayoutCursor::AtVariableName(i) => {
                                    line == LineLocation::VariableDefinition(*i)
                                }
                                _ => false,
                            })
                        })
                        .unwrap_or(false);

                    ui.add(egui::Label::new(
                        egui::RichText::new("let ")
                            .text_style(egui::TextStyle::Monospace)
                            .background_color(egui::Color32::TRANSPARENT),
                    ));
                    Self::with_flashing_frame(ui, name_in_focus, |ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new(self.variable_definitions[i].name())
                                .text_style(egui::TextStyle::Monospace)
                                .strong()
                                .background_color(egui::Color32::TRANSPARENT),
                        ));
                    });
                }
                LineLocation::FinalExpression => {
                    let output_id = outer_context.inspect_number_graph(|g| {
                        let outputs = g.topology().graph_outputs();
                        assert_eq!(outputs.len(), 1);
                        outputs[0].id()
                    });
                    ui.add(egui::Label::new(
                        egui::RichText::new(outer_context.graph_output_name(output_id))
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

            Self::show_child_ast_node(
                ui,
                node,
                graph_state,
                ctx,
                ASTPathBuilder::Root(ast_root),
                &mut cursor_path,
                outer_context,
                &self.variable_definitions,
            );

            match line {
                LineLocation::VariableDefinition(_) => ui.label(","),
                LineLocation::FinalExpression => ui.label("."),
            };
        });

        // TODO: focus to this line if the path was written to
        // Will need to make sure that add_contents writes to it

        let Some(focus) = focus.as_mut() else {
            assert!(cursor_path.is_none());
            return;
        };

        if focus.cursor().line() != line {
            return;
        }

        let (pressed_left, pressed_right) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight),
            )
        });

        if pressed_left || pressed_right {
            focus.close_summon_widget();
        }

        if pressed_left {
            focus.cursor_mut().go_left(self);
        }
        if pressed_right {
            focus.cursor_mut().go_right(self);
        }
    }

    fn show_child_ast_node(
        ui: &mut egui::Ui,
        node: &ASTNode,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
        outer_context: &OuterNumberGraphUiContext,
        variable_definitions: &[VariableDefinition],
    ) {
        let hovering = ui
            .input(|i| i.pointer.hover_pos())
            .and_then(|p| Some(node.is_directly_over(p)))
            .unwrap_or(false);
        Self::with_cursor(ui, path, cursor, hovering, |ui, cursor| {
            let rect = match node.value() {
                ASTNodeValue::Empty => {
                    let r = ui.label("???");
                    r.rect
                }
                ASTNodeValue::Internal(n) => {
                    let r = Self::show_internal_node(
                        ui,
                        n,
                        graph_state,
                        ctx,
                        path,
                        cursor,
                        outer_context,
                        variable_definitions,
                    );
                    r.rect
                }
                ASTNodeValue::Variable(id) => {
                    ui.add(egui::Label::new(
                        egui::RichText::new(
                            find_variable_definition(*id, variable_definitions)
                                .unwrap()
                                .name(),
                        )
                        .code()
                        .color(egui::Color32::WHITE),
                    ))
                    .rect
                }
                ASTNodeValue::GraphInput(giid) => {
                    let name = outer_context.graph_input_name(*giid);
                    let r = ui
                        .add(egui::Label::new(
                            egui::RichText::new(name).code().color(egui::Color32::WHITE),
                        ))
                        .rect;
                    r
                }
            };
            node.set_rect(rect);
        });
    }

    fn show_internal_node(
        ui: &mut egui::Ui,
        node: &InternalASTNode,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        path: ASTPathBuilder,
        cursor: &mut Option<ASTPath>,
        outer_context: &OuterNumberGraphUiContext,
        variable_definitions: &[VariableDefinition],
    ) -> egui::Response {
        let styled_text = |ui: &mut egui::Ui, s: String| -> egui::Response {
            let text = egui::RichText::new(s).code().color(egui::Color32::WHITE);
            ui.add(egui::Label::new(text))
        };

        // TODO: clean this up also

        let ir = ui.horizontal_centered(|ui| {
            let hovering_over_self = ui
                .input(|i| i.pointer.hover_pos())
                .and_then(|p| Some(node.over_self(p)))
                .unwrap_or(false);
            let own_rect = match &node.value() {
                InternalASTNodeValue::Prefix(nsid, expr) => {
                    let r = Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx, outer_context)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        graph_state,
                        ctx,
                        path.push(node, 0),
                        cursor,
                        outer_context,
                        variable_definitions,
                    );
                    r
                }
                InternalASTNodeValue::Infix(expr1, nsid, expr2) => {
                    Self::show_child_ast_node(
                        ui,
                        expr1,
                        graph_state,
                        ctx,
                        path.push(node, 0),
                        cursor,
                        outer_context,
                        variable_definitions,
                    );
                    let r = Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx, outer_context)
                    });
                    Self::show_child_ast_node(
                        ui,
                        expr2,
                        graph_state,
                        ctx,
                        path.push(node, 1),
                        cursor,
                        outer_context,
                        variable_definitions,
                    );
                    r
                }
                InternalASTNodeValue::Postfix(expr, nsid) => {
                    Self::show_child_ast_node(
                        ui,
                        expr,
                        graph_state,
                        ctx,
                        path.push(node, 0),
                        cursor,
                        outer_context,
                        variable_definitions,
                    );
                    Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                        Self::show_number_source_ui(ui, *nsid, graph_state, ctx, outer_context)
                    })
                }
                InternalASTNodeValue::Function(nsid, exprs) => {
                    if exprs.is_empty() {
                        Self::with_cursor(ui, path, cursor, hovering_over_self, |ui, _| {
                            Self::show_number_source_ui(ui, *nsid, graph_state, ctx, outer_context)
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
                                        Self::show_number_source_ui(
                                            ui,
                                            *nsid,
                                            graph_state,
                                            ctx,
                                            outer_context,
                                        )
                                    },
                                );
                                styled_text(ui, "(".to_string());
                                if let Some((last_expr, other_exprs)) = exprs.split_last() {
                                    for (i, expr) in other_exprs.iter().enumerate() {
                                        Self::show_child_ast_node(
                                            ui,
                                            expr,
                                            graph_state,
                                            ctx,
                                            path.push(node, i),
                                            cursor,
                                            outer_context,
                                            variable_definitions,
                                        );
                                        styled_text(ui, ",".to_string());
                                    }
                                    Self::show_child_ast_node(
                                        ui,
                                        last_expr,
                                        graph_state,
                                        ctx,
                                        path.push(node, other_exprs.len()),
                                        cursor,
                                        outer_context,
                                        variable_definitions,
                                    );
                                }
                                styled_text(ui, ")".to_string());
                                r
                            })
                            .inner
                    }
                }
            };

            node.set_self_rect(own_rect);
        });

        ir.response
    }

    fn show_number_source_ui(
        ui: &mut egui::Ui,
        id: NumberSourceId,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        outer_context: &OuterNumberGraphUiContext,
    ) -> egui::Rect {
        let graph_object = outer_context.inspect_number_graph(|numbergraph| {
            numbergraph
                .topology()
                .number_source(id)
                .unwrap()
                .instance_arc()
                .as_graph_object()
        });
        let object_type = graph_object.get_type();
        let object_ui = ctx.ui_factory().get_object_ui(object_type);
        let object_state = ctx.object_ui_states().get_object_data(id);
        ui.horizontal_centered(|ui| {
            object_ui.apply(&graph_object, &object_state, graph_state, ui, ctx);
        })
        .response
        .rect
    }

    fn flashing_highlight_color(ui: &mut egui::Ui) -> egui::Color32 {
        let t = ui.input(|i| i.time);
        let a = (((t - t.floor()) * 2.0 * std::f64::consts::TAU).sin() * 16.0 + 64.0) as u8;
        ui.ctx().request_repaint();
        egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, a)
    }

    fn with_flashing_frame<R, F: FnOnce(&mut egui::Ui) -> R>(
        ui: &mut egui::Ui,
        highlight: bool,
        add_contents: F,
    ) -> egui::InnerResponse<R> {
        let color = if highlight {
            Self::flashing_highlight_color(ui)
        } else {
            egui::Color32::TRANSPARENT
        };
        let frame = egui::Frame::default()
            .inner_margin(2.0)
            .fill(color)
            .stroke(egui::Stroke::new(2.0, color));
        frame.show(ui, add_contents)
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
            let r = Self::with_flashing_frame(ui, highlight, |ui| add_contents(ui, cursor));

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

    pub(crate) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        outer_context: &mut OuterNumberGraphUiContext,
    ) {
        debug_assert!(outer_context
            .inspect_number_graph(|g| { lexical_layout_matches_number_graph(self, g.topology()) }));

        self.handle_summon_widget(
            ui,
            focus,
            object_factory,
            ui_factory,
            object_ui_states,
            outer_context,
        );

        if focus.summon_widget_state().is_none() {
            let cursor = focus.cursor_mut();
            let (pressed_up, pressed_down) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                )
            });
            if pressed_up {
                cursor.go_up(self);
            }
            if pressed_down {
                cursor.go_down(self);
            }

            let pressed_delete =
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));

            if pressed_delete {
                // TODO: debug deleting with cursor over variable name
                delete_from_numbergraph_at_cursor(self, focus.cursor_mut(), outer_context);
                remove_unreferenced_graph_inputs(self, outer_context);
            }
        }

        debug_assert!(outer_context
            .inspect_number_graph(|g| { lexical_layout_matches_number_graph(self, g.topology()) }));
    }

    fn handle_summon_widget(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        outer_context: &mut OuterNumberGraphUiContext,
    ) {
        if focus.cursor().get_node(self).is_none() {
            return;
        }

        let pressed_space_or_tab = ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
        });

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
                    } = e
                    {
                        if *pressed {
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

        if focus.summon_widget_state_mut().is_none() {
            if pressed_space_or_tab || !algebraic_keys_pressed.is_empty() {
                //  open summon widget when space/tab is pressed
                if let Some(node_at_cursor) = focus.cursor().get_node(self) {
                    let mut widget_state = match outer_context {
                        OuterNumberGraphUiContext::SoundNumberInput(sni_ctx) => {
                            build_summon_widget_for_sound_number_input(
                                node_at_cursor.rect().center_bottom(),
                                ui_factory,
                                sni_ctx,
                                focus.cursor().get_variables_in_scope(self),
                            )
                        }
                    };
                    let s = String::from_iter(algebraic_keys_pressed);
                    widget_state.set_text(s);

                    *focus.summon_widget_state_mut() = Some(widget_state);
                } else {
                    *focus.summon_widget_state_mut() = None;
                }
            }
        }

        if let Some(summon_widget_state) = focus.summon_widget_state_mut() {
            if let Some(choice) = summon_widget_state.final_choice() {
                let (summon_value, arguments) = choice;

                debug_assert!(outer_context.inspect_number_graph(|g| {
                    lexical_layout_matches_number_graph(self, g.topology())
                }));

                let (new_node, layout) = match summon_value {
                    NumberSummonValue::NumberSourceType(ns_type) => self
                        .create_new_number_source_from_type(
                            ns_type,
                            arguments,
                            object_factory,
                            ui_factory,
                            object_ui_states,
                            outer_context,
                        )
                        .unwrap(),
                    NumberSummonValue::SoundNumberSource(snsid) => {
                        let node;
                        {
                            let outer_context = match outer_context {
                                OuterNumberGraphUiContext::SoundNumberInput(ctx) => ctx,
                            };
                            let giid = if let Some(giid) =
                                outer_context.find_graph_id_for_number_source(snsid)
                            {
                                giid
                            } else {
                                let giid = outer_context.connect_to_number_source(snsid);
                                giid
                            };
                            node = ASTNode::new(ASTNodeValue::GraphInput(giid));
                        }
                        (node, NumberSourceLayout::Function)
                    }
                    NumberSummonValue::Variable(variable_id) => (
                        ASTNode::new(ASTNodeValue::Variable(variable_id)),
                        NumberSourceLayout::Function,
                    ),
                    NumberSummonValue::Constant(constant_value) => {
                        let (node, layout) = self
                            .create_new_number_source_from_type(
                                Constant::TYPE,
                                arguments
                                    .add_or_replace(&Constant::ARG_VALUE, constant_value as f64),
                                object_factory,
                                ui_factory,
                                object_ui_states,
                                outer_context,
                            )
                            .unwrap();
                        (node, layout)
                    }
                };
                let num_children = new_node.num_children();
                insert_to_numbergraph_at_cursor(self, focus.cursor_mut(), new_node, outer_context);

                debug_assert!(outer_context.inspect_number_graph(|g| {
                    lexical_layout_matches_number_graph(self, g.topology())
                }));

                let cursor_path = focus.cursor_mut().path_mut().unwrap();
                match layout {
                    NumberSourceLayout::Prefix => cursor_path.go_into(0),
                    NumberSourceLayout::Infix => cursor_path.go_into(0),
                    NumberSourceLayout::Postfix => cursor_path.go_into(0),
                    NumberSourceLayout::Function => {
                        if num_children > 0 {
                            cursor_path.go_into(0);
                        }
                    }
                }
                focus.close_summon_widget();
            }
        }
    }

    fn create_new_number_source_from_type(
        &self,
        ns_type: ObjectType,
        arguments: ParsedArguments,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        outer_context: &mut OuterNumberGraphUiContext,
    ) -> Result<(ASTNode, NumberSourceLayout), String> {
        let new_object = outer_context
            .edit_number_graph(|numbergraph| {
                object_factory.create_from_args(ns_type.name(), numbergraph, arguments.clone())
            })
            .unwrap();

        let new_object = match new_object {
            Ok(o) => o,
            Err(_) => {
                return Err(format!(
                    "Failed to create number object of type {}",
                    ns_type.name()
                ));
            }
        };

        let new_ui_state = ui_factory
            .create_state_from_arguments(&new_object, arguments)
            .map_err(|e| format!("Failed to create ui state: {:?}", e))?;

        let num_inputs = outer_context.inspect_number_graph(|numbergraph| {
            numbergraph
                .topology()
                .number_source(new_object.id())
                .unwrap()
                .number_inputs()
                .len()
        });
        let child_nodes: Vec<ASTNode> = (0..num_inputs)
            .map(|_| ASTNode::new(ASTNodeValue::Empty))
            .collect();
        let internal_node = make_internal_node(new_object.id(), &new_ui_state, child_nodes);
        let node = ASTNode::new(ASTNodeValue::Internal(Box::new(internal_node)));

        let layout = new_ui_state.layout();

        object_ui_states.set_object_data(new_object.id(), new_ui_state);

        Ok((node, layout))
    }

    pub(super) fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(&self, mut f: F) {
        for vardef in &self.variable_definitions {
            vardef.value().visit(
                ASTPathBuilder::Root(ASTRoot::VariableDefinition(vardef.id(), vardef.name())),
                &mut f,
            );
        }
        self.final_expression
            .visit(ASTPathBuilder::Root(ASTRoot::FinalExpression), &mut f);
    }

    pub(super) fn visit_mut<F: FnMut(&mut ASTNode, ASTPathBuilder)>(&mut self, mut f: F) {
        for vardef in &mut self.variable_definitions {
            let VariableDefinition { id, name, value } = vardef;
            value.visit_mut(
                ASTPathBuilder::Root(ASTRoot::VariableDefinition(*id, name)),
                &mut f,
            );
        }
        self.final_expression
            .visit_mut(ASTPathBuilder::Root(ASTRoot::FinalExpression), &mut f);
    }

    pub(crate) fn cleanup(
        &mut self,
        topology: &NumberGraphTopology,
        object_ui_states: &NumberObjectUiStates,
    ) {
        fn visitor(
            node: &mut ASTNode,
            expected_target: Option<NumberTarget>,
            variable_definitions: &[VariableDefinition],
            topo: &NumberGraphTopology,
        ) {
            let actual_target = node.indirect_target(variable_definitions);
            if expected_target == actual_target {
                // TODO: if the node is a reference to a variable,
                // note the expected target and use it to visit
                // the variable definition later

                if let Some(internal_node) = node.as_internal_node_mut() {
                    let nsid = internal_node.number_source_id();
                    let expected_inputs = topo.number_source(nsid).unwrap().number_inputs();
                    let expected_targets: Vec<Option<NumberTarget>> = expected_inputs
                        .iter()
                        .map(|niid| topo.number_input(*niid).unwrap().target())
                        .collect();

                    if internal_node.num_children() != expected_inputs.len() {
                        if let InternalASTNodeValue::Function(_, cs) = internal_node.value_mut() {
                            // see notes below
                            todo!("Allocate new AST nodes for function arguments")
                        } else {
                            panic!("A number source changed number of inputs whose ui doesn't support that");
                        }
                    }
                    match internal_node.value_mut() {
                        InternalASTNodeValue::Prefix(_, c) => {
                            visitor(c, expected_targets[0], variable_definitions, topo)
                        }
                        InternalASTNodeValue::Infix(c1, _, c2) => {
                            visitor(c1, expected_targets[0], variable_definitions, topo);
                            visitor(c2, expected_targets[1], variable_definitions, topo);
                        }
                        InternalASTNodeValue::Postfix(c, _) => {
                            visitor(c, expected_targets[0], variable_definitions, topo)
                        }
                        InternalASTNodeValue::Function(_, cs) => {
                            for (c, exp_tgt) in cs.iter_mut().zip(expected_targets) {
                                visitor(c, exp_tgt, variable_definitions, topo)
                            }
                        }
                    }
                }
            } else {
                // actual node target doesn't match
                match expected_target {
                    Some(NumberTarget::GraphInput(giid)) => {
                        *node = ASTNode::new(ASTNodeValue::GraphInput(giid))
                    }
                    Some(NumberTarget::Source(nsid)) => {
                        // TODO:
                        // - if an existing (direct) variable definition exists for the source,
                        //   create a reference to that variable
                        // - otherwise, if the the number source is already referenced by
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

        let graph_outputs = topology.graph_outputs();
        assert_eq!(graph_outputs.len(), 1);
        let graph_output = &graph_outputs[0];

        visitor(
            &mut self.final_expression,
            graph_output.target(),
            &self.variable_definitions,
            topology,
        );

        // TODO: after having gathered expected targets for variable definitions,
        // visit those to confirm that they match

        // TODO: create variable definitions for any unreferenced number sources
    }
}
