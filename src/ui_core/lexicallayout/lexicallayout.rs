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
        lexicallayout::ast::{ASTNodeValue, InternalASTNodeValue},
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
            cursor: LexicalLayoutCursor::new(),
            summon_widget_state: None,
        }
    }

    pub(super) fn cursor(&self) -> &LexicalLayoutCursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut LexicalLayoutCursor {
        &mut self.cursor
    }

    pub(super) fn summon_widget_state_mut(
        &mut self,
    ) -> &mut Option<SummonWidgetState<NumberSummonValue>> {
        &mut self.summon_widget_state
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

#[derive(Clone)]
pub(super) struct LexicalLayoutCursor {
    line: usize,
    path: ASTPath,
}

impl LexicalLayoutCursor {
    pub(super) fn new() -> LexicalLayoutCursor {
        LexicalLayoutCursor {
            line: 0,
            path: ASTPath::new(Vec::new()),
        }
    }

    pub(super) fn path(&self) -> &ASTPath {
        &self.path
    }

    pub(super) fn path_mut(&mut self) -> &mut ASTPath {
        &mut self.path
    }
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

        LexicalLayout {
            variable_definitions: variable_assignments,
            final_expression,
            variable_id_generator,
        }
    }

    pub(super) fn final_expression(&self) -> &ASTNode {
        &self.final_expression
    }

    pub(crate) fn show(
        &mut self,
        ui: &mut egui::Ui,
        result_label: &str,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        mut focus: Option<&mut LexicalLayoutFocus>,
        outer_context: &OuterNumberGraphUiContext,
    ) {
        let variable_definitions = &self.variable_definitions;
        let num_variable_definitions = variable_definitions.len();
        let final_expression = &self.final_expression;

        let mut cursor = focus.as_mut().and_then(|f| Some(f.cursor_mut()));

        // TODO: clean this up, way to many redundant arguments being passed around

        ui.vertical(|ui| {
            for (i, var_def) in variable_definitions.iter().enumerate() {
                let line_number = i;
                Self::show_line(
                    ui,
                    var_def.value(),
                    &mut cursor,
                    line_number,
                    |ui, cursor, node| {
                        ui.horizontal(|ui| {
                            // TODO: make this and other text pretty
                            ui.label(format!("{} = ", var_def.name()));
                            Self::show_child_ast_node(
                                ui,
                                node,
                                graph_state,
                                ctx,
                                ASTPathBuilder::Root(ASTRoot::VariableDefinition(var_def)),
                                cursor,
                                outer_context,
                                variable_definitions,
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
                &mut cursor,
                line_number,
                |ui, cursor, node| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} = ", result_label));
                        Self::show_child_ast_node(
                            ui,
                            node,
                            graph_state,
                            ctx,
                            ASTPathBuilder::Root(ASTRoot::FinalExpression),
                            cursor,
                            outer_context,
                            variable_definitions,
                        );
                        ui.label(".");
                    });
                },
            );
        });

        if let Some(summon_widget_state) = focus
            .and_then(|f| Some(f.summon_widget_state_mut().as_mut()))
            .flatten()
        {
            let summon_widget = SummonWidget::new(summon_widget_state);
            ui.add(summon_widget);
            // TODO: ?
        }
    }

    fn show_line<F: FnOnce(&mut egui::Ui, &mut Option<ASTPath>, &ASTNode)>(
        ui: &mut egui::Ui,
        node: &ASTNode,
        cursor: &mut Option<&mut LexicalLayoutCursor>,
        line_number: usize,
        add_contents: F,
    ) {
        let mut cursor_path = if let Some(cursor) = cursor {
            if cursor.line == line_number {
                Some(cursor.path.clone())
            } else {
                None
            }
        } else {
            None
        };

        add_contents(ui, &mut cursor_path, node);

        if let Some(mut path) = cursor_path {
            if let Some(cursor) = cursor {
                let (pressed_left, pressed_right) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft),
                        i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight),
                    )
                });

                if pressed_left {
                    path.go_left(node);
                }
                if pressed_right {
                    path.go_right(node);
                }

                **cursor = LexicalLayoutCursor {
                    line: line_number,
                    path,
                };
            }
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
                    // TODO: show cursor?
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
        egui::Color32::from_rgba_unmultiplied(0xff, 0xff, 0xff, a)
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

    pub(crate) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        outer_context: &mut OuterNumberGraphUiContext,
    ) {
        // TODO: consider filtering egui's InputState's vec of inputs
        // and consuming key presses from there

        {
            let cursor = focus.cursor_mut();
            let (pressed_up, pressed_down) = ui.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                )
            });
            if pressed_up {
                cursor.line = cursor.line.saturating_sub(1);
                cursor.path.clear();
            }
            if pressed_down {
                cursor.line = (cursor.line + 1).min(self.variable_definitions.len());
                cursor.path.clear();
            }

            let pressed_delete =
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Delete));

            if pressed_delete {
                delete_from_numbergraph_at_cursor(self, focus.cursor_mut(), outer_context);
            }
        }

        self.handle_summon_widget(
            ui,
            focus,
            object_factory,
            ui_factory,
            object_ui_states,
            outer_context,
        );

        // TODO: create a summon widget similar to that used in flosion_ui.
        // start typing to gather a list of candidate number sources.
        // Ideally the experience should have minimal overhead typing,
        // e.g. typing something like "sin x + 2 * b" should result in sin(x + (2 * b))
        // with all intermediate values nicely built up. This might require knowing
        // operator precedence and doing funny things with the cursor but hopefully not.
        // Intermediates with cursor:
        // (no input yet)   -> _
        //                   ^
        // "sin "           -> sin(_)
        //                       ^
        // "x "             -> sin(x)
        //                         ^
        // "+ "             -> sin(x + _)           NOTE: so typing an infix operator nests the selected ast node
        //                             ^            as the left child and places the cursor on the right child?
        //
        // "2 "             -> sin(x + 2)
        //                             ^
        // "* "             -> sin(x + (2 * _))     NOTE: operator precedence could be applied here to determine
        //                                  ^       whether or not to place a node inside or around the parent(s)
        //
        // "b "             -> sin(x + (2 * b))
        //                                  ^
        //
        // left a bunch     -> sin(x + (2 * b))     NOTE: could press home also
        //                     ^
        // "^ ""            -> sin(x + (2 * b))^_
        //                                      ^
        // "1 "             -> sin(x + (2 * b))^1
        //                                      ^
        // "/"              -> sin(x + (2 * b))^(1/_)
        //                                         ^
        // "2 "             -> sin(x + (2 * b))^(1/2)
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
                let node_at_cursor = self.get_node_at_cursor(&focus.cursor());
                let mut widget_state = match outer_context {
                    OuterNumberGraphUiContext::SoundNumberInput(sni_ctx) => {
                        build_summon_widget_for_sound_number_input(
                            node_at_cursor.rect().center_bottom(),
                            ui_factory,
                            sni_ctx,
                        )
                    }
                };
                let s = String::from_iter(algebraic_keys_pressed);
                widget_state.set_text(s);

                *focus.summon_widget_state_mut() = Some(widget_state);
            }
        }

        if let Some(summon_widget_state) = focus.summon_widget_state_mut() {
            if let Some(choice) = summon_widget_state.final_choice() {
                let (summon_value, arguments) = choice;
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
                                outer_context.connect_to_number_source(snsid)
                            };
                            node = ASTNode::new(ASTNodeValue::GraphInput(giid));
                        }
                        (node, NumberSourceLayout::Function)
                    }
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

                let cursor = focus.cursor_mut();
                match layout {
                    NumberSourceLayout::Prefix => cursor.path.go_into(0),
                    NumberSourceLayout::Infix => cursor.path.go_into(0),
                    NumberSourceLayout::Postfix => cursor.path.go_into(0),
                    NumberSourceLayout::Function => {
                        if num_children > 0 {
                            cursor.path.go_into(0);
                        }
                    }
                }
                *focus.summon_widget_state_mut() = None;
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

    pub(super) fn get_cursor_root(&self, cursor: &LexicalLayoutCursor) -> Option<ASTRoot> {
        if cursor.path.at_beginning() {
            if cursor.line < self.variable_definitions.len() {
                Some(ASTRoot::VariableDefinition(
                    &self.variable_definitions[cursor.line],
                ))
            } else if cursor.line == self.variable_definitions.len() {
                Some(ASTRoot::FinalExpression)
            } else {
                panic!("Cursor out of range")
            }
        } else {
            None
        }
    }

    pub(super) fn get_node_at_cursor(&self, cursor: &LexicalLayoutCursor) -> &ASTNode {
        if cursor.line < self.variable_definitions.len() {
            self.variable_definitions[cursor.line]
                .value()
                .get_along_path(cursor.path.steps())
        } else if cursor.line == self.variable_definitions.len() {
            self.final_expression.get_along_path(cursor.path.steps())
        } else {
            panic!("Invalid line number")
        }
    }

    pub(super) fn set_node_at_cursor(&mut self, cursor: &LexicalLayoutCursor, value: ASTNode) {
        if cursor.line < self.variable_definitions.len() {
            self.variable_definitions[cursor.line]
                .value_mut()
                .set_along_path(cursor.path.steps(), value);
        } else if cursor.line == self.variable_definitions.len() {
            self.final_expression
                .set_along_path(cursor.path.steps(), value);
        } else {
            panic!("Invalid line number")
        }
    }

    pub(super) fn visit<F: FnMut(&ASTNode, ASTPathBuilder)>(&self, mut f: F) {
        for vardef in &self.variable_definitions {
            vardef.value().visit(
                ASTPathBuilder::Root(ASTRoot::VariableDefinition(vardef)),
                &mut f,
            );
        }
        self.final_expression
            .visit(ASTPathBuilder::Root(ASTRoot::FinalExpression), &mut f);
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
        ) {
            let actual_target = node.indirect_target(variable_definitions);
            if expected_target == actual_target {
                // nice

                if let Some(internal_node) = node.internal_node() {
                    match internal_node.value() {
                        // TODO: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                        AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                        InternalASTNodeValue::Prefix(_, _) => todo!(),
                        InternalASTNodeValue::Infix(_, _, _) => todo!(),
                        InternalASTNodeValue::Postfix(_, _) => todo!(),
                        InternalASTNodeValue::Function(_, _) => todo!(),
                    }
                }
            }
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
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA
                    todo!()
                }
                None => {
                    *node = ASTNode::new(ASTNodeValue::Empty);
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
        );
    }
}
