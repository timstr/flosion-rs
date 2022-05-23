use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedDynamicSoundProcessor},
    objects::whitenoise::WhiteNoise,
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{ObjectUi, ObjectWindow, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type WrapperType = WrappedDynamicSoundProcessor<WhiteNoise>;

    fn ui(
        &self,
        id: ObjectId,
        _wrapper: &WrappedDynamicSoundProcessor<WhiteNoise>,
        graph_state: &mut GraphUITools,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("WhiteNoise");
            ui.add(SoundOutputWidget::new(id, "Output", graph_state));
        });
    }
}
