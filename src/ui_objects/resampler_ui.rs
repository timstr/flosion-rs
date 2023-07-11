use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::resampler::Resampler,
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
pub struct ResamplerUi {}

impl ObjectUi for ResamplerUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = NoUIState;
    fn ui(
        &self,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        graph_tools: &mut SoundGraphUIState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        data: ConcreteSoundObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(resampler.id(), "Resampler", data.color)
            .add_sound_input(resampler.input.id())
            .add_number_input(resampler.speed_ratio.id(), "Speed Ratio")
            .show(ui, ctx, graph_tools);
    }
}
