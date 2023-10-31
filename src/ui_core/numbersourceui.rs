use eframe::egui;

use crate::core::number::numbersource::NumberSourceId;

use super::{numbergraphuicontext::NumberGraphUiContext, numbergraphuistate::NumberGraphUiState};

pub enum DisplayStyle {
    Framed,
    Frameless,
}

pub struct NumberSourceUi {
    source_id: NumberSourceId,
    label: Option<String>,
    display_style: DisplayStyle,
}

impl NumberSourceUi {
    pub fn new_named(
        source_id: NumberSourceId,
        label: String,
        display_style: DisplayStyle,
    ) -> NumberSourceUi {
        NumberSourceUi {
            source_id,
            label: Some(label),
            display_style,
        }
    }

    pub fn new_unnamed(source_id: NumberSourceId, display_style: DisplayStyle) -> NumberSourceUi {
        NumberSourceUi {
            source_id,
            label: None,
            display_style,
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
        let show_impl = |ui: &mut egui::Ui| {
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
        };

        match self.display_style {
            DisplayStyle::Framed => {
                let frame = egui::Frame::default()
                    .fill(egui::Color32::from_rgb(0, 128, 0))
                    .rounding(5.0)
                    .inner_margin(2.0);
                frame.show(ui, show_impl);
            }
            DisplayStyle::Frameless => {
                let frame = egui::Frame::default()
                    .fill(egui::Color32::BLACK)
                    .rounding(2.0)
                    .inner_margin(1.0);
                frame.show(ui, show_impl);
            }
        }
    }
}
