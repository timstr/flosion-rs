use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        numberinputplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WaveGenerator>;
    type StateType = ();

    fn ui(
        &self,
        wavgen: DynamicSoundProcessorHandle<WaveGenerator>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(wavgen.id(), "WaveGenerator", data.color)
            .add_number_input(
                wavgen.amplitude.id(),
                "amplitude",
                PlotConfig::new()
                    .linear_vertical_range(-1.0..=1.0)
                    .with_respect_to(wavgen.phase.id(), 0.0..=1.0),
            )
            .add_number_input(wavgen.frequency.id(), "frequency", PlotConfig::new())
            .add_number_source(wavgen.phase.id(), "phase")
            .add_number_source(wavgen.time.id(), "time")
            .show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["wavegenerator"]
    }
}
