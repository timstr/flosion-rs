use crate::{
    core::graphobject::ObjectId,
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{ObjectUi, ObjectWindow, SoundInputWidget},
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type ObjectType = Dac;
    fn ui(
        &self,
        id: ObjectId,
        object: &Dac,
        graph_state: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
    ) {
        ObjectWindow::new_sound_processor(id.as_sound_processor_id().unwrap()).show(
            ui.ctx(),
            |ui| {
                ui.label("Dac");
                // ui.separator();
                ui.label(if object.is_playing() {
                    "Playing"
                } else {
                    "Paused"
                });
                ui.add(SoundInputWidget::new(object.input().id(), graph_state));
            },
        );
    }
}
