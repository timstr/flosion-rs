use eframe::egui;
use hashstash::{Stash, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::{
    core::sound::{
        expression::ProcessorExpressionLocation, soundgraph::SoundGraph,
        soundinput::SoundInputLocation, soundprocessor::SoundProcessorId,
    },
    ui_core::{
        expressiongraphuicontext::OuterProcessorExpressionContext,
        expressiongraphuistate::ExpressionUiCollection, factories::Factories,
        graph_properties::GraphProperties, history::SnapshotFlag,
        lexicallayout::lexicallayout::LexicalLayoutFocus, soundgraphuinames::SoundGraphUiNames,
        soundobjectpositions::SoundObjectPositions, stackedlayout::stackedlayout::StackedLayout,
    },
};

use super::draganddrop::DragDropSubject;

struct DirectionsToGo {
    go_up: bool,
    go_down: bool,
    go_in: bool,
    go_out: bool,
}

impl DirectionsToGo {
    fn nowhere() -> DirectionsToGo {
        DirectionsToGo {
            go_up: false,
            go_down: false,
            go_in: false,
            go_out: false,
        }
    }

    fn filter_keypresses(&self, ui: &mut egui::Ui) -> DirectionsToGo {
        ui.input_mut(|i| DirectionsToGo {
            go_up: self.go_up && i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
            go_down: self.go_down && i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            go_in: self.go_in && i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            go_out: self.go_out && i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
        })
    }

    fn draw_highlights(&self, ui: &mut egui::Ui, rect: egui::Rect, faint: bool) {
        let color_hi = egui::Color32::from_white_alpha(if faint { 128 } else { 255 });
        let color_lo = egui::Color32::TRANSPARENT;

        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(3.0),
            egui::Stroke::new(2.0, color_hi),
        );

        if self.go_up {
            // Draw a fading white trapezoid above the top edge
            let w = 10.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_top(), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(w, -w), color_lo);
            mesh.colored_vertex(rect.right_top() + egui::vec2(-w, -w), color_lo);
            mesh.colored_vertex(rect.right_top(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        if self.go_down {
            // Draw a fading white trapezoid below the top edge
            let w = 10.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_bottom(), color_hi);
            mesh.colored_vertex(rect.left_bottom() + egui::vec2(w, w), color_lo);
            mesh.colored_vertex(rect.right_bottom() + egui::vec2(-w, w), color_lo);
            mesh.colored_vertex(rect.right_bottom(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        if self.go_in {
            // Draw a trimmed glowing corner going inside from the bottom right
            let w = 30.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.right_bottom(), color_hi);
            mesh.colored_vertex(rect.right_bottom() + egui::vec2(-2.0 * w, 0.0), color_hi);
            mesh.colored_vertex(rect.right_bottom() + egui::vec2(-w, -w), color_lo);
            mesh.colored_vertex(rect.right_bottom() + egui::vec2(0.0, -2.0 * w), color_hi);

            mesh.colored_vertex(rect.right_bottom(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            ui.painter().add(mesh);
        }

        if self.go_out {
            // Draw a trimmed glowing corner going outside from the top left
            let w = 15.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_top(), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(2.0 * w, 0.0), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(w, -w), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(-w, -w), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(-w, w), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(0.0, 2.0 * w), color_hi);

            mesh.colored_vertex(rect.right_bottom(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            mesh.add_triangle(0, 3, 4);
            mesh.add_triangle(0, 4, 5);
            ui.painter().add(mesh);
        }
    }
}

pub(crate) enum KeyboardNavInteraction {
    // AroundGroup(???)
    // OnJumperCable(???)
    AroundSoundProcessor(SoundProcessorId),
    AroundProcessorPlug(SoundProcessorId),
    AroundInputSocket(SoundInputLocation),
    AroundExpression(ProcessorExpressionLocation),
    InsideExpression(ProcessorExpressionLocation, LexicalLayoutFocus),
}

impl KeyboardNavInteraction {
    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        graph: &mut SoundGraph,
        layout: &StackedLayout,
        positions: &SoundObjectPositions,
        expression_uis: &mut ExpressionUiCollection,
        factories: &Factories,
        stash: &Stash,
        names: &SoundGraphUiNames,
        properties: &GraphProperties,
        snapshot_flag: &SnapshotFlag,
    ) {
        let rect;
        let mut allowed_dirs = DirectionsToGo::nowhere();
        let mut faint_highlight = false;

        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => {
                rect = positions.find_processor(*spid).unwrap().body_rect;
                let proc_data = graph.sound_processor(*spid).unwrap();
                let last_input = proc_data.input_locations().last().cloned();

                let first_expr: Option<ProcessorExpressionLocation> = positions
                    .processor_expressions_top_down(*spid)
                    .first()
                    .cloned();

                allowed_dirs.go_up = last_input.is_some();
                allowed_dirs.go_down = true;
                allowed_dirs.go_in = first_expr.is_some();

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    // go the processor's last input, if it has any inputs
                    if let Some(last_input) = last_input {
                        *self = KeyboardNavInteraction::AroundInputSocket(last_input);
                        snapshot_flag.request_snapshot();
                    }
                } else if requested_dirs.go_down {
                    // go to the processor's plug
                    *self = KeyboardNavInteraction::AroundProcessorPlug(*spid);
                    snapshot_flag.request_snapshot();
                } else if requested_dirs.go_in {
                    // go to the processor's first expression

                    if let Some(eid) = first_expr {
                        *self = KeyboardNavInteraction::AroundExpression(eid);
                        snapshot_flag.request_snapshot();
                    }
                }
            }
            KeyboardNavInteraction::AroundProcessorPlug(spid) => {
                rect = positions
                    .drag_drop_subjects()
                    .get(&DragDropSubject::Plug(*spid))
                    .unwrap()
                    .clone();
                let proc_below = layout.processor_below(*spid);

                allowed_dirs.go_up = true;
                allowed_dirs.go_down = proc_below.is_some();

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    // go to the processor
                    *self = KeyboardNavInteraction::AroundSoundProcessor(*spid);
                    snapshot_flag.request_snapshot();
                } else if requested_dirs.go_down {
                    // if there's a processor below, go to its first input
                    if let Some(proc_below) = proc_below {
                        let first_input = graph
                            .sound_processor(proc_below)
                            .unwrap()
                            .input_locations()
                            .first()
                            .cloned()
                            .unwrap();
                        *self = KeyboardNavInteraction::AroundInputSocket(first_input);
                        snapshot_flag.request_snapshot();
                    } else {
                        // TODO: ???
                    }
                }
            }
            KeyboardNavInteraction::AroundInputSocket(siid) => {
                rect = positions
                    .drag_drop_subjects()
                    .get(&DragDropSubject::Socket(*siid))
                    .unwrap()
                    .clone();
                let owner = siid.processor();
                let other_inputs = graph.sound_processor(owner).unwrap().input_locations();
                let index = other_inputs.iter().position(|id| *id == *siid).unwrap();

                allowed_dirs.go_up = index > 0 || !layout.is_top_of_group(owner);
                allowed_dirs.go_down = true;

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    if index == 0 {
                        // go to the target processor if there is one
                        if let Some(proc_above) = layout.processor_above(owner) {
                            *self = KeyboardNavInteraction::AroundProcessorPlug(proc_above);
                            snapshot_flag.request_snapshot();
                        } else {
                            // TODO: ???
                        }
                    } else {
                        // go the previous input
                        *self = KeyboardNavInteraction::AroundInputSocket(other_inputs[index - 1]);
                        snapshot_flag.request_snapshot();
                    }
                } else if requested_dirs.go_down {
                    if index + 1 == other_inputs.len() {
                        // go to the processor
                        *self = KeyboardNavInteraction::AroundSoundProcessor(owner);
                        snapshot_flag.request_snapshot();
                    } else {
                        // go the the next input
                        *self = KeyboardNavInteraction::AroundInputSocket(other_inputs[index + 1]);
                        snapshot_flag.request_snapshot();
                    }
                }
            }
            KeyboardNavInteraction::AroundExpression(eid) => {
                rect = positions.expressions().get(eid).unwrap().clone();

                let other_exprs: Vec<ProcessorExpressionLocation> =
                    positions.processor_expressions_top_down(eid.processor());
                let index = other_exprs.iter().position(|id| *id == *eid).unwrap();

                allowed_dirs.go_up = index > 0;
                allowed_dirs.go_down = (index + 1) < other_exprs.len();
                allowed_dirs.go_in = true;
                allowed_dirs.go_out = true;

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    if index > 0 {
                        *self = KeyboardNavInteraction::AroundExpression(other_exprs[index - 1]);
                        snapshot_flag.request_snapshot();
                    }
                } else if requested_dirs.go_down {
                    if index + 1 < other_exprs.len() {
                        *self = KeyboardNavInteraction::AroundExpression(other_exprs[index + 1]);
                        snapshot_flag.request_snapshot();
                    }
                } else if requested_dirs.go_in {
                    *self =
                        KeyboardNavInteraction::InsideExpression(*eid, LexicalLayoutFocus::new());
                    snapshot_flag.request_snapshot();
                } else if requested_dirs.go_out {
                    *self = KeyboardNavInteraction::AroundSoundProcessor(eid.processor());
                    snapshot_flag.request_snapshot();
                }
            }
            KeyboardNavInteraction::InsideExpression(eid, ll_focus) => {
                rect = positions.expressions().get(eid).unwrap().clone();
                faint_highlight = true;

                allowed_dirs.go_out = true;

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_out {
                    *self = KeyboardNavInteraction::AroundExpression(*eid);
                    snapshot_flag.request_snapshot();
                } else {
                    let (expr_ui_state, ll) = expression_uis.get_mut(*eid).unwrap();

                    // TODO: why does this sometimes not find a node?
                    // Answer: because the cursor is over a variable name.
                    if let Some(rect) = ll_focus.cursor().get_bounding_rect(ll) {
                        ui.painter().rect_stroke(
                            rect,
                            egui::Rounding::same(3.0),
                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                        );
                    }

                    let time_axis = layout.find_group(eid.processor()).unwrap().time_axis();

                    let available_inputs = properties.available_inputs(eid.processor()).unwrap();
                    let available_arguments = properties.available_arguments(*eid).unwrap();

                    graph
                        .sound_processor_mut(eid.processor())
                        .unwrap()
                        .with_expression_mut(eid.expression(), |expr| {
                            let (mapping, expr_graph) = expr.parts_mut();

                            let outer_context = OuterProcessorExpressionContext::new(
                                *eid,
                                mapping,
                                names,
                                time_axis,
                                available_inputs,
                                available_arguments,
                                snapshot_flag,
                            );

                            ll.handle_keypress(
                                ui,
                                ll_focus,
                                expr_graph,
                                factories,
                                stash,
                                expr_ui_state.object_states_mut(),
                                &mut outer_context.into(),
                            );
                        });
                }
            }
        };

        allowed_dirs.draw_highlights(ui, rect, faint_highlight);

        // TODO: handle arrow keys / enter / escape to change focus, tab to summon,
        // delete to delete, shortcuts for extracting/moving/reconnecting processors???
    }

    /// Returns true iff all graph ids referenced by the keyboard focus
    /// refer to objects that exist in the given graph
    pub(crate) fn is_valid(&self, graph: &SoundGraph) -> bool {
        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => graph.contains(spid),
            KeyboardNavInteraction::AroundProcessorPlug(spid) => graph.contains(spid),
            KeyboardNavInteraction::AroundInputSocket(siid) => graph.contains(siid),
            // TODO: check that expression also exists
            KeyboardNavInteraction::AroundExpression(eid) => graph.contains(eid.processor()),
            KeyboardNavInteraction::InsideExpression(eid, _) => graph.contains(eid.processor()),
        }
    }
}

impl Stashable for KeyboardNavInteraction {
    fn stash(&self, stasher: &mut Stasher) {
        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => {
                stasher.u8(0);
                spid.stash(stasher);
            }
            KeyboardNavInteraction::AroundProcessorPlug(spid) => {
                stasher.u8(1);
                spid.stash(stasher);
            }
            KeyboardNavInteraction::AroundInputSocket(input_loc) => {
                stasher.u8(2);
                input_loc.stash(stasher);
            }
            KeyboardNavInteraction::AroundExpression(expr_loc) => {
                stasher.u8(3);
                expr_loc.stash(stasher);
            }
            KeyboardNavInteraction::InsideExpression(expr_loc, ll_focus) => {
                stasher.u8(4);
                expr_loc.stash(stasher);
                ll_focus.stash(stasher);
            }
        }
    }
}

impl Unstashable for KeyboardNavInteraction {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let kni = match unstasher.u8()? {
            0 => {
                KeyboardNavInteraction::AroundSoundProcessor(SoundProcessorId::unstash(unstasher)?)
            }
            1 => KeyboardNavInteraction::AroundProcessorPlug(SoundProcessorId::unstash(unstasher)?),
            2 => KeyboardNavInteraction::AroundInputSocket(SoundInputLocation::unstash(unstasher)?),
            3 => KeyboardNavInteraction::AroundExpression(ProcessorExpressionLocation::unstash(
                unstasher,
            )?),
            4 => KeyboardNavInteraction::InsideExpression(
                ProcessorExpressionLocation::unstash(unstasher)?,
                LexicalLayoutFocus::unstash(unstasher)?,
            ),
            _ => panic!(),
        };

        Ok(kni)
    }
}
