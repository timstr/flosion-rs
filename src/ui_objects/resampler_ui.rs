use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::resampler::Resampler,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl ObjectUi for ResamplerUi {
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = NoUIState;
    fn ui(
        &self,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        data: ObjectUiData<NoUIState>,
    ) {
        ObjectWindow::new_sound_processor(resampler.id(), "Resampler", data.color)
            // .add_left_peg(resampler.input.id(), "Input")
            // .add_top_peg(&resampler.speed_ratio, "Speed Ratio")
            // .add_right_peg(resampler.id(), "Output")
            .show(ui.ctx(), graph_tools);
    }
}
