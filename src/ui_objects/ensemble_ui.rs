use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::ensemble::Ensemble,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct EnsembleUi {}

impl ObjectUi for EnsembleUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Ensemble>;
    type StateType = ();

    fn ui(
        &self,
        ensemble: DynamicSoundProcessorHandle<Ensemble>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        ProcessorUi::new(ensemble.id(), "Ensemble", data.color)
            .add_sound_input(ensemble.input.id(), "input")
            .add_number_input(ensemble.frequency_in.id(), "frequency_in")
            .add_number_input(ensemble.frequency_spread.id(), "frequency_spread")
            .add_number_source(ensemble.voice_frequency.id(), "voice_frequency")
            .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["ensemble"]
    }
}
