use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::resampler::Resampler,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl SoundObjectUi for ResamplerUi {
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = ();
    fn ui(
        &self,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&resampler, "Resampler")
            .add_sound_input(resampler.get().input.id(), "input", sound_graph)
            .add_expression(resampler.get().speed_ratio.id(), "speed", PlotConfig::new())
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["resampler"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
