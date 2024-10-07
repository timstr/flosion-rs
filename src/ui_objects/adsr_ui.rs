use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::WhateverSoundProcessorHandle},
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
    type HandleType = WhateverSoundProcessorHandle<ADSR>;
    type StateType = ();

    fn ui(
        &self,
        adsr: WhateverSoundProcessorHandle<ADSR>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        let id = adsr.id();

        let mut adsr = adsr.get_mut();
        let adsr: &mut ADSR = &mut adsr;

        let ADSR {
            input,
            attack_time,
            decay_time,
            sustain_level,
            release_time,
        } = adsr;

        ProcessorUi::new(id, "ADSR")
            .add_sound_input(input.id(), "input")
            .add_expression(attack_time, "attack_time", PlotConfig::new())
            .add_expression(decay_time, "decay_time", PlotConfig::new())
            .add_expression(sustain_level, "sustain_level", PlotConfig::new())
            .add_expression(release_time, "release_time", PlotConfig::new())
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
