use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::adsr::ADSR,
    ui_core::{
        expressionplot::PlotConfig,
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ADSRUi {}

impl ObjectUi for ADSRUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<ADSR>;
    type StateType = ();
    fn ui(
        &self,
        adsr: DynamicSoundProcessorHandle<ADSR>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&adsr, "ADSR", data.color)
            .add_sound_input(adsr.input.id(), "input", sound_graph)
            .add_expression(adsr.attack_time.id(), "attack_time", PlotConfig::new())
            .add_expression(adsr.decay_time.id(), "decay_time", PlotConfig::new())
            .add_expression(adsr.sustain_level.id(), "sustain_level", PlotConfig::new())
            .add_expression(adsr.release_time.id(), "release_time", PlotConfig::new())
            .show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["adsr"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
