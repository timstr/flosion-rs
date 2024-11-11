use crate::{
    core::sound::{argument::ProcessorArgumentLocation, soundprocessor::SoundProcessorWithId},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl SoundObjectUi for WaveGeneratorUi {
    type ObjectType = SoundProcessorWithId<WaveGenerator>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        wavgen: &mut SoundProcessorWithId<WaveGenerator>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new("WaveGenerator")
            .add_expression(
                &wavgen.amplitude,
                "amplitude",
                PlotConfig::new()
                    .linear_vertical_range(-1.0..=1.0)
                    .with_respect_to(
                        // TODO: ew, why not just `wavgen.phase`?
                        ProcessorArgumentLocation::new(wavgen.id(), wavgen.phase.id()),
                        0.0..=1.0,
                    ),
            )
            .add_expression(&wavgen.frequency, "frequency", PlotConfig::new())
            .add_argument(&wavgen.phase, "phase")
            .show(wavgen, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["wavegenerator"]
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
