use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::resampler::Resampler,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl ObjectUi for ResamplerUi {
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = NoUIState;
    fn ui(
        &self,
        id: ObjectId,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id)
            .add_left_peg(resampler.input.id(), "Input")
            .add_top_peg(&resampler.speed_ratio, "Speed Ratio")
            .add_right_peg(resampler.id(), "Output")
            .show(ui.ctx(), graph_tools, |ui, _graph_tools| {
                ui.label("Resampler");
            });
    }
}
