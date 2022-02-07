use crate::{objects::wavegenerator::WaveGenerator, ui_core::object_ui::ObjectUi};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type ObjectType = WaveGenerator;
    fn ui(&self, _object: &WaveGenerator, ui: &mut eframe::egui::Ui) {
        ui.label("WaveGenerator Ui");
    }
}
