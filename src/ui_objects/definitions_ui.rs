use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::definitions::Definitions,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct DefinitionsUi {}

impl SoundObjectUi for DefinitionsUi {
    type HandleType = DynamicSoundProcessorHandle<Definitions>;

    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        definitions: DynamicSoundProcessorHandle<Definitions>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&definitions, "Definitions")
            .add_expression(definitions.get().expression.id(), "a", PlotConfig::new())
            .add_argument(definitions.get().argument.id(), "a")
            .add_sound_input(definitions.get().sound_input.id(), "input", graph)
            .show_with(
                ui,
                ctx,
                graph_ui_state,
                graph,
                |_ui, _uistate, _sound_graph| {
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

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
