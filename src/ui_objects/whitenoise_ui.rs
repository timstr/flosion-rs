use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::whitenoise::WhiteNoise,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type HandleType = DynamicSoundProcessorHandle<WhiteNoise>;
    type StateType = NoUIState;

    fn ui(
        &self,
        whitenoise: DynamicSoundProcessorHandle<WhiteNoise>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        ObjectWindow::new_sound_processor(whitenoise.id())
            .add_right_peg(whitenoise.id(), "Output")
            .show(ui.ctx(), graph_tools, |ui, _graph_tools| {
                ui.label("WhiteNoise");
            });
    }
}
