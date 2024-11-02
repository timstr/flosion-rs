use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::definitions::Definitions,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct DefinitionsUi {}

impl SoundObjectUi for DefinitionsUi {
    type ObjectType = SoundProcessorWithId<Definitions>;
    type StateType = NoObjectUiState;

    fn ui<'a, 'b>(
        &self,
        definitions: &mut SoundProcessorWithId<Definitions>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new(definitions.id(), "Definitions")
            .add_expression(&definitions.expression, "a", PlotConfig::new())
            .add_argument(definitions.argument.id(), "a")
            .add_sound_input(definitions.sound_input.id(), "input")
            .show_with(
                definitions,
                ui,
                ctx,
                graph_ui_state,
                |_definitions, _ui, _uistate| {
                    // TODO: controls to rename source
                    // TODO: buttons to add/remove terms
                },
            )
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["definitions"]
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
