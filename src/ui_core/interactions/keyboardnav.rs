use std::collections::HashSet;

use eframe::egui;

use crate::{
    core::sound::{
        expression::SoundExpressionId, soundgraph::SoundGraph,
        soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
        soundprocessor::SoundProcessorId,
    },
    ui_core::{
        expressiongraphuicontext::OuterProcessorExpressionContext,
        expressiongraphuistate::ExpressionUiCollection, flosion_ui::Factories,
        lexicallayout::lexicallayout::LexicalLayoutFocus, soundgraphuinames::SoundGraphUiNames,
        soundobjectpositions::SoundObjectPositions, stackedlayout::stackedlayout::SoundGraphLayout,
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
            let width = 10.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_top(), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(width, -width), color_lo);
            mesh.colored_vertex(rect.right_top() + egui::vec2(-width, -width), color_lo);
            mesh.colored_vertex(rect.right_top(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        if self.go_down {
            // Draw a fading white trapezoid below the top edge
            let width = 10.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_bottom(), color_hi);
            mesh.colored_vertex(rect.left_bottom() + egui::vec2(width, width), color_lo);
            mesh.colored_vertex(rect.right_bottom() + egui::vec2(-width, width), color_lo);
            mesh.colored_vertex(rect.right_bottom(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        if self.go_in {
            // Draw a trimmed glowing corner going inside from the top left
            let width = 15.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_top(), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(width, 0.0), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(2.0 * width, width), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(width, width), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(width, 2.0 * width), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(0.0, width), color_hi);

            mesh.colored_vertex(rect.right_bottom(), color_hi);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            mesh.add_triangle(0, 3, 4);
            mesh.add_triangle(0, 4, 5);
            ui.painter().add(mesh);
        }

        if self.go_out {
            // Draw a trimmed glowing corner going outside from the top left
            let width = 15.0;
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_top(), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(2.0 * width, 0.0), color_hi);
            mesh.colored_vertex(rect.left_top() + egui::vec2(width, -width), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(-width, -width), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(-width, width), color_lo);
            mesh.colored_vertex(rect.left_top() + egui::vec2(0.0, 2.0 * width), color_hi);

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
    AroundInputSocket(SoundInputId),
    AroundExpression(SoundExpressionId),
    InsideExpression(SoundExpressionId, LexicalLayoutFocus),
}

impl KeyboardNavInteraction {
    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        graph: &mut SoundGraph,
        layout: &SoundGraphLayout,
        positions: &SoundObjectPositions,
        expression_uis: &mut ExpressionUiCollection,
        factories: &Factories,
        names: &SoundGraphUiNames,
    ) {
        let rect;
        let mut allowed_dirs = DirectionsToGo::nowhere();
        let mut faint_highlight = false;

        let topo = graph.topology();

        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => {
                rect = positions.find_processor(*spid).unwrap().rect;
                let proc_data = topo.sound_processor(*spid).unwrap();
                let last_input = proc_data.sound_inputs().last();

                let first_expr: Option<SoundExpressionId> = positions
                    .expressions()
                    .values()
                    .iter()
                    .cloned()
                    .find(|eid| topo.expression(*eid).unwrap().owner() == *spid);

                allowed_dirs.go_up = last_input.is_some();
                allowed_dirs.go_down = true;
                allowed_dirs.go_in = first_expr.is_some();

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    // go the processor's last input, if it has any inputs
                    if let Some(last_input) = last_input {
                        *self = KeyboardNavInteraction::AroundInputSocket(*last_input);
                    }
                } else if requested_dirs.go_down {
                    // go to the processor's plug
                    *self = KeyboardNavInteraction::AroundProcessorPlug(*spid);
                } else if requested_dirs.go_in {
                    // go to the processor's first expression

                    if let Some(eid) = first_expr {
                        *self = KeyboardNavInteraction::AroundExpression(eid);
                    }
                }
            }
            KeyboardNavInteraction::AroundProcessorPlug(spid) => {
                rect = positions
                    .drag_drop_subjects()
                    .position(&DragDropSubject::Plug(*spid))
                    .unwrap();
                let proc_below = layout.processor_below(*spid);

                allowed_dirs.go_up = true;
                allowed_dirs.go_down = proc_below.is_some();

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    // go to the processor
                    *self = KeyboardNavInteraction::AroundSoundProcessor(*spid);
                } else if requested_dirs.go_down {
                    // if there's a processor below, go to its first input
                    if let Some(proc_below) = proc_below {
                        let first_input = topo
                            .sound_processor(proc_below)
                            .unwrap()
                            .sound_inputs()
                            .first()
                            .unwrap();
                        *self = KeyboardNavInteraction::AroundInputSocket(*first_input);
                    } else {
                        // TODO: ???
                    }
                }
            }
            KeyboardNavInteraction::AroundInputSocket(siid) => {
                rect = positions
                    .drag_drop_subjects()
                    .position(&DragDropSubject::Socket(*siid))
                    .unwrap();
                let owner = topo.sound_input(*siid).unwrap().owner();
                let other_inputs = topo.sound_processor(owner).unwrap().sound_inputs();
                let index = other_inputs.iter().position(|id| *id == *siid).unwrap();

                allowed_dirs.go_up = index > 0 || !layout.is_top_of_group(owner);
                allowed_dirs.go_down = true;

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    if index == 0 {
                        // go to the target processor if there is one
                        if let Some(proc_above) = layout.processor_above(owner) {
                            *self = KeyboardNavInteraction::AroundProcessorPlug(proc_above);
                        } else {
                            // TODO: ???
                        }
                    } else {
                        // go the previous input
                        *self = KeyboardNavInteraction::AroundInputSocket(other_inputs[index - 1]);
                    }
                } else if requested_dirs.go_down {
                    if index + 1 == other_inputs.len() {
                        // go to the processor
                        *self = KeyboardNavInteraction::AroundSoundProcessor(owner);
                    } else {
                        // go the the next input
                        *self = KeyboardNavInteraction::AroundInputSocket(other_inputs[index + 1]);
                    }
                }
            }
            KeyboardNavInteraction::AroundExpression(eid) => {
                rect = positions.expressions().position(eid).unwrap();

                let owner = topo.expression(*eid).unwrap().owner();

                let other_exprs: Vec<SoundExpressionId> = positions
                    .expressions()
                    .values()
                    .iter()
                    .cloned()
                    .filter(|eid| topo.expression(*eid).unwrap().owner() == owner)
                    .collect();
                let index = other_exprs.iter().position(|id| *id == *eid).unwrap();

                allowed_dirs.go_up = index > 0;
                allowed_dirs.go_down = (index + 1) < other_exprs.len();
                allowed_dirs.go_in = true;
                allowed_dirs.go_out = true;

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_up {
                    if index > 0 {
                        *self = KeyboardNavInteraction::AroundExpression(other_exprs[index - 1]);
                    }
                } else if requested_dirs.go_down {
                    if index + 1 < other_exprs.len() {
                        *self = KeyboardNavInteraction::AroundExpression(other_exprs[index + 1])
                    }
                } else if requested_dirs.go_in {
                    *self =
                        KeyboardNavInteraction::InsideExpression(*eid, LexicalLayoutFocus::new())
                } else if requested_dirs.go_out {
                    *self = KeyboardNavInteraction::AroundSoundProcessor(owner);
                }
            }
            KeyboardNavInteraction::InsideExpression(eid, ll_focus) => {
                rect = positions.expressions().position(eid).unwrap();
                faint_highlight = true;

                allowed_dirs.go_out = true;

                let requested_dirs = allowed_dirs.filter_keypresses(ui);

                if requested_dirs.go_out {
                    *self = KeyboardNavInteraction::AroundExpression(*eid);
                } else {
                    let owner = topo.expression(*eid).unwrap().owner();

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

                    let time_axis = layout.find_group(owner).unwrap().time_axis();

                    // HACK no args 4 u
                    // TODO: cache this for the whole graph using RevisedProperty
                    let available_arguments = HashSet::new();

                    let outer_context = OuterProcessorExpressionContext::new(
                        *eid,
                        owner,
                        names,
                        time_axis,
                        &available_arguments,
                    );

                    ll.handle_keypress(
                        ui,
                        ll_focus,
                        graph,
                        factories.expression_objects(),
                        factories.expression_uis(),
                        expr_ui_state.object_states_mut(),
                        &outer_context.into(),
                    );
                }
            }
        };

        allowed_dirs.draw_highlights(ui, rect, faint_highlight);

        // TODO: handle arrow keys / enter / escape to change focus, tab to summon,
        // delete to delete, shortcuts for extracting/moving/reconnecting processors???
    }

    /// Returns true iff all graph ids referenced by the keyboard focus
    /// refer to objects that exist in the given topology
    pub(crate) fn is_valid(&self, topo: &SoundGraphTopology) -> bool {
        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => topo.contains(spid),
            KeyboardNavInteraction::AroundProcessorPlug(spid) => topo.contains(spid),
            KeyboardNavInteraction::AroundInputSocket(siid) => topo.contains(siid),
            KeyboardNavInteraction::AroundExpression(eid) => topo.contains(eid),
            KeyboardNavInteraction::InsideExpression(eid, _) => topo.contains(eid),
        }
    }
}
