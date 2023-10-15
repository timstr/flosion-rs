use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::resampler::Resampler,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl ObjectUi for ResamplerUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = ();
    fn ui(
        &self,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        ProcessorUi::new(resampler.id(), "Resampler", data.color)
            .add_sound_input(resampler.input.id(), "input")
            .add_number_input(resampler.speed_ratio.id(), "speed")
            .show(ui, ctx, ui_state);
    }
}
