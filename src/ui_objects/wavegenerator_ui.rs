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
            .add_number_input(wavgen.amplitude.id(), "amplitude")
            .add_number_input(wavgen.frequency.id(), "frequency")
            .add_number_source(wavgen.phase.id(), "phase")
            .add_number_source(wavgen.time.id(), "time")
            .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["wavegenerator"]
    }
}
