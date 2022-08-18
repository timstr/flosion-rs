use crate::{
    core::{graphobject::ObjectId, soundprocessor::SoundProcessorHandle},
    objects::whitenoise::WhiteNoise,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type WrapperType = SoundProcessorHandle<WhiteNoise>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        _wrapper: &SoundProcessorHandle<WhiteNoise>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("WhiteNoise");
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
        });
    }
}
