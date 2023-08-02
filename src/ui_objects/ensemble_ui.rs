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
        ctx: &SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        ProcessorUi::new(ensemble.id(), "Ensemble", data.color)
            // .add_left_peg(ensemble.input.id(), "Input")
            // .add_left_peg(&ensemble.voice_frequency, "Voice Frequency")
            // .add_top_peg(&ensemble.frequency_in, "Frequency In")
            // .add_top_peg(&ensemble.frequency_spread, "Frequency Spread")
            // .add_right_peg(ensemble.id(), "Output")
            .add_sound_input(ensemble.input.id())
            .add_number_input(ensemble.frequency_in.id(), "Frequency In")
            .add_number_input(ensemble.frequency_spread.id(), "Frequency Spread")
            .show(ui, ctx, ui_state);
    }
}
