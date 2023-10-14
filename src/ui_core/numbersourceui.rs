use eframe::egui;

use crate::core::number::numbersource::NumberSourceId;

use super::{numbergraphuicontext::NumberGraphUiContext, numbergraphuistate::NumberGraphUiState};

pub struct NumberSourceUi {
    source_id: NumberSourceId,
    label: Option<String>,
}

impl NumberSourceUi {
    pub fn new_named(source_id: NumberSourceId, label: String) -> NumberSourceUi {
        NumberSourceUi {
            source_id,
            label: Some(label),
        }
    }

    pub fn new_unnamed(source_id: NumberSourceId) -> NumberSourceUi {
        NumberSourceUi {
            source_id,
            label: None,
        }
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        ctx: &mut NumberGraphUiContext,
        ui_state: &mut NumberGraphUiState,
    ) {
        self.show_with(ui, ctx, ui_state, |_ui, _ui_state| {});
    }

    pub fn show_with<F: FnOnce(&mut egui::Ui, &mut NumberGraphUiState)>(
        self,
        ui: &mut egui::Ui,
        ctx: &mut NumberGraphUiContext,
        ui_state: &mut NumberGraphUiState,
        add_contents: F,
    ) {
        let frame = egui::Frame::default()
            .fill(egui::Color32::from_rgba_unmultiplied(0, 255, 0, 32))
            // .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(64)))
            .rounding(5.0)
            .inner_margin(2.0);
        let r = frame.show(ui, |ui| {
            if let Some(label) = self.label {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(label)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .sense(egui::Sense::click())
                    .wrap(false),
                );
            }
            add_contents(ui, ui_state);
        });
        // let r = r.response.interact(egui::Sense::click());
        // let r = ui.interact(r.response.rect, r.response.id, egui::Sense::click());
        // let r = r
        //     .response
        //     .union(ui.interact(r.response.rect, r.response.id, egui::Sense::click()));
        // if r.clicked() {
        //     println!("Clicked");
        // }
    }
}
