use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::resampler::Resampler,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl SoundObjectUi for ResamplerUi {
    type ObjectType = SoundProcessorWithId<Resampler>;
    type StateType = NoObjectUiState;
    fn ui(
        &self,
        resampler: &mut SoundProcessorWithId<Resampler>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new("Resampler")
            .add_sound_input(&resampler.input, "input")
            .add_expression(&resampler.speed_ratio, "speed", PlotConfig::new())
            .show(resampler, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["resampler"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<NoObjectUiState, ()> {
        Ok(NoObjectUiState)
    }
}
