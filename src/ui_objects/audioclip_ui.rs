use eframe::egui;

use crate::{
    core::graphobject::ObjectId,
    objects::audioclip::AudioClip,
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{ObjectUi, ObjectWindow, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct AudioClipUi {}

impl ObjectUi for AudioClipUi {
    type ObjectType = AudioClip;

    fn ui(
        &self,
        id: ObjectId,
        _object: &AudioClip,
        graph_state: &mut GraphUITools,
        ui: &mut egui::Ui,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("AudioClip");
            ui.add(SoundOutputWidget::new(id, graph_state));
        })
    }
}
