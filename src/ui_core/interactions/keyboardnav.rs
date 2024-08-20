use eframe::egui;

use crate::{
    core::sound::{
        expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
        soundprocessor::SoundProcessorId,
    },
    ui_core::{
        lexicallayout::lexicallayout::LexicalLayoutFocus,
        soundobjectpositions::SoundObjectPositions,
        stackedlayout::interconnect::{InputSocket, ProcessorPlug},
    },
};

pub(crate) enum KeyboardNavInteraction {
    AroundSoundProcessor(SoundProcessorId),
    OnSoundProcessorName(SoundProcessorId),
    AroundProcessorPlug(ProcessorPlug),
    AroundInputSocket(InputSocket),
    AroundExpression(SoundExpressionId),
    InsideExpression(SoundExpressionId, LexicalLayoutFocus),
}

impl KeyboardNavInteraction {
    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        positions: &SoundObjectPositions,
    ) {
        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => {
                let rect = positions.find_processor(*spid).unwrap().rect;
                ui.painter().rect_stroke(
                    rect,
                    egui::Rounding::same(3.0),
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
            }
            KeyboardNavInteraction::OnSoundProcessorName(spid) => todo!(),
            KeyboardNavInteraction::AroundProcessorPlug(plug) => todo!(),
            KeyboardNavInteraction::AroundInputSocket(socket) => todo!(),
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
            KeyboardNavInteraction::AroundProcessorPlug(p) => topo.contains(p.processor),
            KeyboardNavInteraction::AroundInputSocket(s) => topo.contains(s.input),
            KeyboardNavInteraction::AroundExpression(eid) => topo.contains(eid),
            KeyboardNavInteraction::InsideExpression(eid, _) => topo.contains(eid),
        }
    }
}
