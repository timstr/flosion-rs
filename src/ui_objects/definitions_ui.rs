use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::definitions::Definitions,
    ui_core::{
        numberinputplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData, soundprocessorui::ProcessorUi,
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
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&definitions, "Definitions", data.color)
            .add_number_input(definitions.number_input.id(), "a", PlotConfig::new())
            .add_number_source(definitions.number_source.id(), "a")
            .add_sound_input(definitions.sound_input.id(), "input", graph)
            .show_with(ui, ctx, ui_state, graph, |ui, _uistate, _sound_graph| {
                // TODO: controls to rename source
                // TODO: buttons to add/remove terms
            })
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["definitions"]
    }
}
