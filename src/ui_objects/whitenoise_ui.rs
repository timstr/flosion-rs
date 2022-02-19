use crate::{
    core::graphobject::ObjectId,
    objects::whitenoise::WhiteNoise,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{ObjectUi, ObjectWindow, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type ObjectType = WhiteNoise;
    fn ui(
        &self,
        id: ObjectId,
        _object: &WhiteNoise,
        graph_state: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("WhiteNoise");
            ui.add(SoundOutputWidget::new(id, graph_state));
        });
    }
}
