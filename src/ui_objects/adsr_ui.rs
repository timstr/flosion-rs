use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::adsr::ADSR,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ADSRUi {}

impl SoundObjectUi for ADSRUi {
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

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
