use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::resampler::Resampler,
    ui_core::{
        numberinputplot::PlotConfig,
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl ObjectUi for ResamplerUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = ();
    fn ui(
        &self,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&resampler, "Resampler", data.color)
            .add_sound_input(resampler.input.id(), "input", sound_graph)
            .add_number_input(resampler.speed_ratio.id(), "speed", PlotConfig::new())
            .show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["resampler"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
