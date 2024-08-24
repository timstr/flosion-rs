use eframe::egui;

use crate::{
    core::sound::{
        expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
    ui_core::{
        lexicallayout::lexicallayout::LexicalLayoutFocus,
        soundobjectpositions::SoundObjectPositions, stackedlayout::stackedlayout::SoundGraphLayout,
    },
};

use super::draganddrop::DragDropSubject;

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
        topo: &SoundGraphTopology,
        layout: &SoundGraphLayout,
        positions: &SoundObjectPositions,
    ) {
        // TODO: deduplicate!
        // interact, modify, gather drawing info, THEN draw.
        // one `match self` should suffice.

        let (pressed_up, pressed_down, pressed_enter, pressed_escape) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });

        let rect;
        let can_go_up;
        let can_go_down;
        let can_go_in;
        // let can_go_out;

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

                can_go_up = last_input.is_some();
                can_go_down = true;
                can_go_in = first_expr.is_some();

                if pressed_up {
                    // go the processor's last input, if it has any inputs
                    if let Some(last_input) = last_input {
                        *self = KeyboardNavInteraction::AroundInputSocket(*last_input);
                    }
                } else if pressed_down {
                    // go to the processor's plug
                    *self = KeyboardNavInteraction::AroundProcessorPlug(*spid);
                } else if pressed_enter {
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
                can_go_up = true;
                can_go_down = proc_below.is_some();
                can_go_in = false;

                if pressed_up {
                    // go to the processor
                    *self = KeyboardNavInteraction::AroundSoundProcessor(*spid);
                } else if pressed_down {
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
                can_go_up = index > 0 || !layout.is_top_of_group(owner);
                can_go_down = true;
                can_go_in = false;

                if pressed_up {
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
                } else if pressed_down {
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

                can_go_up = index > 0;
                can_go_down = (index + 1) < other_exprs.len();
                can_go_in = true;

                if pressed_up {
                    if index > 0 {
                        *self = KeyboardNavInteraction::AroundExpression(other_exprs[index - 1]);
                    }
                } else if pressed_down {
                    if index + 1 < other_exprs.len() {
                        *self = KeyboardNavInteraction::AroundExpression(other_exprs[index + 1])
                    }
                } else if pressed_enter {
                    *self =
                        KeyboardNavInteraction::InsideExpression(*eid, LexicalLayoutFocus::new())
                } else if pressed_escape {
                    *self = KeyboardNavInteraction::AroundSoundProcessor(owner);
                }
            }
            KeyboardNavInteraction::InsideExpression(_, _) => todo!(),
        };

        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(3.0),
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );

        let glow_width = 10.0;

        if can_go_up {
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_top(), egui::Color32::WHITE);
            mesh.colored_vertex(
                rect.left_top() + egui::vec2(glow_width, -glow_width),
                egui::Color32::TRANSPARENT,
            );
            mesh.colored_vertex(
                rect.right_top() + egui::vec2(-glow_width, -glow_width),
                egui::Color32::TRANSPARENT,
            );
            mesh.colored_vertex(rect.right_top(), egui::Color32::WHITE);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        if can_go_down {
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(rect.left_bottom(), egui::Color32::WHITE);
            mesh.colored_vertex(
                rect.left_bottom() + egui::vec2(glow_width, glow_width),
                egui::Color32::TRANSPARENT,
            );
            mesh.colored_vertex(
                rect.right_bottom() + egui::vec2(-glow_width, glow_width),
                egui::Color32::TRANSPARENT,
            );
            mesh.colored_vertex(rect.right_bottom(), egui::Color32::WHITE);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        if can_go_in {
            let glow_width = 30.0;
            let mut mesh = egui::Mesh::default();
            // Top left corner
            mesh.colored_vertex(rect.left_top(), egui::Color32::WHITE);
            mesh.colored_vertex(
                rect.left_top() + egui::vec2(glow_width, 0.0),
                egui::Color32::TRANSPARENT,
            );
            mesh.colored_vertex(
                rect.left_top() + egui::vec2(glow_width, glow_width),
                egui::Color32::TRANSPARENT,
            );
            mesh.colored_vertex(
                rect.left_top() + egui::vec2(0.0, glow_width),
                egui::Color32::TRANSPARENT,
            );

            mesh.colored_vertex(rect.right_bottom(), egui::Color32::WHITE);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(2, 3, 0);
            ui.painter().add(mesh);
        }

        // TODO: handle arrow keys / enter / escape to change focus, tab to summon,
        // delete to delete, shortcuts for extracting/moving/reconnecting processors???
    }

    pub(crate) fn expression_focus(
        &mut self,
        id: SoundExpressionId,
    ) -> Option<&mut LexicalLayoutFocus> {
        match self {
            KeyboardNavInteraction::InsideExpression(snid, focus) => {
                if *snid == id {
                    Some(focus)
                } else {
                    None
                }
            }
            _ => None,
        }
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
