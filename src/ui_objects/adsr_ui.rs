use crate::{
    core::{
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::adsr::ADSR,
    ui_core::{
        expressionplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
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
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&adsr, "ADSR")
            .add_sound_input(adsr.input.id(), "input", sound_graph)
            .add_expression(adsr.attack_time.id(), "attack_time", PlotConfig::new())
            .add_expression(adsr.decay_time.id(), "decay_time", PlotConfig::new())
            .add_expression(adsr.sustain_level.id(), "sustain_level", PlotConfig::new())
            .add_expression(adsr.release_time.id(), "release_time", PlotConfig::new())
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["adsr"]
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
