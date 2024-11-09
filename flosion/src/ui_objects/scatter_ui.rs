use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::SoundProcessorWithId},
    objects::scatter::Scatter,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ScatterUi {}

impl SoundObjectUi for ScatterUi {
    type ObjectType = SoundProcessorWithId<Scatter>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        scatter: &mut SoundProcessorWithId<Scatter>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        // TODO: controls to change number of voices
        // TODO: controls to change variables and type of
        // distribution and parameter per variable

        ProcessorUi::new(scatter.id(), "Scatter")
            .add_sound_input(scatter.sound_input.id(), "input")
            .add_expression(&scatter.parameter, "parameter", PlotConfig::new())
            .add_argument(scatter.value.id(), "value")
            .show(scatter, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["scatter"]
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
