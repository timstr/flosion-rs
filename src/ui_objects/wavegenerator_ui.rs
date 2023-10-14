use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WaveGenerator>;
    type StateType = ();

    fn ui(
        &self,
        wavgen: DynamicSoundProcessorHandle<WaveGenerator>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        ProcessorUi::new(wavgen.id(), "WaveGenerator", data.color)
            .add_number_input(wavgen.amplitude.id(), "Amplitude")
            .add_number_input(wavgen.frequency.id(), "Frequency")
            .add_number_source(wavgen.phase.id(), "Phase")
            .add_number_source(wavgen.time.id(), "Time")
            .show(ui, ctx, ui_state);
    }
}
