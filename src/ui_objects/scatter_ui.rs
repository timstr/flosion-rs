use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::scatter::Scatter,
    ui_core::{
        numberinputplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ScatterUi {}

impl ObjectUi for ScatterUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Scatter>;
    type StateType = ();

    fn ui(
        &self,
        scatter: DynamicSoundProcessorHandle<Scatter>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        // TODO: controls to change number of voices
        // TODO: controls to change variables and type of
        // distribution and parameter per variable

        ProcessorUi::new(&scatter, "Scatter", data.color)
            .add_sound_input(scatter.sound_input.id(), "input", sound_graph)
            .add_number_input(scatter.parameter.id(), "parameter", PlotConfig::new())
            .add_number_source(scatter.value.id(), "value")
            .show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["ensemble"]
    }
}
