use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::ensemble::Ensemble,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct EnsembleUi {}

impl ObjectUi for EnsembleUi {
    type HandleType = DynamicSoundProcessorHandle<Ensemble>;
    type StateType = NoUIState;

    fn ui(
        &self,
        ensemble: DynamicSoundProcessorHandle<Ensemble>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        data: ObjectUiData<NoUIState>,
    ) {
        ObjectWindow::new_sound_processor(ensemble.id(), "Ensemble", data.color)
            .add_left_peg(ensemble.input.id(), "Input")
            .add_left_peg(&ensemble.voice_frequency, "Voice Frequency")
            .add_top_peg(&ensemble.frequency_in, "Frequency In")
            .add_top_peg(&ensemble.frequency_spread, "Frequency Spread")
            .add_right_peg(ensemble.id(), "Output")
            .show(ui.ctx(), graph_tools);
    }
}
