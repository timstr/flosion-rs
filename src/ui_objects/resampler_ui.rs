use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::resampler::Resampler,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ProcessorUi},
        ui_context::UiContext,
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
        ctx: &UiContext,
        data: ObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(resampler.id(), "Resampler", data.color)
            .add_sound_input(resampler.input.id())
            .add_number_input(resampler.speed_ratio.id())
            .show(ui, ctx, graph_tools);
    }
}
