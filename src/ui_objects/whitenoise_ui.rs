use eframe::egui;

use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::whitenoise::WhiteNoise,
    ui_core::{
        object_ui::{NoUIState, ObjectUi},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUIState,
        soundobjectuistate::ConcreteSoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WhiteNoise>;
    type StateType = NoUIState;

    fn ui(
        &self,
        whitenoise: DynamicSoundProcessorHandle<WhiteNoise>,
        graph_tools: &mut SoundGraphUIState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        data: ConcreteSoundObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(whitenoise.id(), "WhiteNoise", data.color)
            // .add_right_peg(whitenoise.id(), "Output")
            .show(ui, ctx, graph_tools);
    }
}
