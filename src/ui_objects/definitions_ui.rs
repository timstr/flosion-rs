use crate::{
    core::{
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::definitions::Definitions,
    ui_core::{
        expressionplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct DefinitionsUi {}

impl ObjectUi for DefinitionsUi {
    type GraphUi = SoundGraphUi;

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
            .add_expression(definitions.expression.id(), "a", PlotConfig::new())
            .add_argument(definitions.argument.id(), "a")
            .add_sound_input(definitions.sound_input.id(), "input", graph)
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

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: ObjectInitialization,
    ) -> Result<(), ()> {
        Ok(())
    }
}
