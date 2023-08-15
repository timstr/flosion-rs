use eframe::egui;

use crate::core::sound::soundnumberinput::SoundNumberInputId;

use super::{
    lexicallayout::LexicalLayout, numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::NumberGraphUiState,
};

pub(super) struct SoundNumberInputUi {
    number_input_id: SoundNumberInputId,
}

impl SoundNumberInputUi {
    pub(super) fn new(id: SoundNumberInputId) -> SoundNumberInputUi {
        SoundNumberInputUi {
            number_input_id: id,
        }
    }

    pub(super) fn show(
        self,
        ui: &mut egui::Ui,
        result_label: &str,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
    ) {
        // TODO:
        // (now) simple frame containing all number sources in lexical ordering
        // (later) expandable/collapsible popup window with full layout
        let frame = egui::Frame::default()
            .fill(egui::Color32::BLACK)
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(64)));
        frame.show(ui, |ui| {
            // TODO: store layout in ui state
            let layout = LexicalLayout::generate(ctx.topology());
            layout.show(ui, result_label, graph_state, ctx);
        });
    }
}
