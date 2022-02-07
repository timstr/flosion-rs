use crate::{objects::dac::Dac, ui_core::object_ui::ObjectUi};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type ObjectType = Dac;
    fn ui(&self, object: &Dac, ui: &mut eframe::egui::Ui) {
        ui.label("Dac Ui");
        ui.label(if object.is_playing() {
            "playing"
        } else {
            "not playing"
        });
    }
}
