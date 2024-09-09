use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl SoundObjectUi for WaveGeneratorUi {
    type HandleType = DynamicSoundProcessorHandle<WaveGenerator>;
    type StateType = ();

    fn ui(
        &self,
        wavgen: DynamicSoundProcessorHandle<WaveGenerator>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&wavgen, "WaveGenerator")
            .add_expression(
                wavgen.get().amplitude.id(),
                "amplitude",
                PlotConfig::new()
                    .linear_vertical_range(-1.0..=1.0)
                    // TODO: why is this not working?
                    .with_respect_to(wavgen.get().phase.id(), 0.0..=1.0),
            )
            .add_expression(wavgen.get().frequency.id(), "frequency", PlotConfig::new())
            .add_argument(wavgen.get().phase.id(), "phase")
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["wavegenerator"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
