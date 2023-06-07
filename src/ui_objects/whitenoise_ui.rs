use eframe::egui;

use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::whitenoise::WhiteNoise,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ProcessorUi},
        ui_context::UiContext,
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
        ui: &mut egui::Ui,
        ctx: &UiContext,
        data: ObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(whitenoise.id(), "WhiteNoise", data.color)
            // .add_right_peg(whitenoise.id(), "Output")
            .show(ui, ctx, graph_tools);
    }
}
