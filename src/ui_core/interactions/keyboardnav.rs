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
    AroundSoundProcessor(SoundProcessorId),
    OnSoundProcessorName(SoundProcessorId),
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
        let (pressed_up, pressed_down) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
            )
        });

        let rect = match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => {
                positions.find_processor(*spid).unwrap().rect
            }
            KeyboardNavInteraction::OnSoundProcessorName(_) => todo!(),
            KeyboardNavInteraction::AroundProcessorPlug(spid) => positions
                .drag_drop_subjects()
                .position(&DragDropSubject::Plug(*spid))
                .unwrap(),
            KeyboardNavInteraction::AroundInputSocket(siid) => positions
                .drag_drop_subjects()
                .position(&DragDropSubject::Socket(*siid))
                .unwrap(),
            KeyboardNavInteraction::AroundExpression(_) => todo!(),
            KeyboardNavInteraction::InsideExpression(_, _) => todo!(),
        };

        // TODO: visual cues about what can be done, e.g. highlights/fades
        // pointing up and down to suggest the up and down arrow keys
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(3.0),
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );

        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => {
                if pressed_up {
                    // go the processor's last input, if it has any inputs
                    if let Some(last_input) =
                        topo.sound_processor(*spid).unwrap().sound_inputs().last()
                    {
                        *self = KeyboardNavInteraction::AroundInputSocket(*last_input);
                    }
                } else if pressed_down {
                    // go to the processor's plug
                    *self = KeyboardNavInteraction::AroundProcessorPlug(*spid);
                }
            }
            KeyboardNavInteraction::OnSoundProcessorName(spid) => todo!(),
            KeyboardNavInteraction::AroundProcessorPlug(spid) => {
                if pressed_up {
                    // go to the processor
                    *self = KeyboardNavInteraction::AroundSoundProcessor(*spid);
                } else if pressed_down {
                    // if there's a processor below, go to its first input
                    if let Some(proc_below) = layout.processor_below(*spid) {
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
                let owner = topo.sound_input(*siid).unwrap().owner();
                let other_inputs = topo.sound_processor(owner).unwrap().sound_inputs();
                let index = other_inputs.iter().position(|id| *id == *siid).unwrap();

                if pressed_up {
                    if index == 0 {
                        // go to the target processor if there is one
                        if let Some(proc_above) = layout.processor_above(owner) {
                            *self = KeyboardNavInteraction::AroundSoundProcessor(proc_above);
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
            KeyboardNavInteraction::AroundExpression(expr) => todo!(),
            KeyboardNavInteraction::InsideExpression(expr, focus) => todo!(),
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
            KeyboardNavInteraction::OnSoundProcessorName(spid) => topo.contains(spid),
            KeyboardNavInteraction::AroundProcessorPlug(spid) => topo.contains(spid),
            KeyboardNavInteraction::AroundInputSocket(siid) => topo.contains(siid),
            KeyboardNavInteraction::AroundExpression(eid) => topo.contains(eid),
            KeyboardNavInteraction::InsideExpression(eid, _) => topo.contains(eid),
        }
    }
}
