pub(crate) use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::WhateverSoundProcessorHandle},
    objects::output::Output,
    ui_core::{
        arguments::ParsedArguments, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectui::SoundObjectUi,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct OutputUi {}

impl SoundObjectUi for OutputUi {
    type HandleType = WhateverSoundProcessorHandle<Output>;
    type StateType = ();
    fn ui(
        &self,
        output: WhateverSoundProcessorHandle<Output>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(output.id(), "Output")
            .add_sound_input(output.get().input.id(), "input")
            .show_with(
                ui,
                ctx,
                graph_ui_state,
                sound_graph,
                |ui, _ui_state, _sound_graph| {
                    if ui
                        .add(egui::Button::new("Start over").wrap_mode(egui::TextWrapMode::Extend))
                        .clicked()
                    {
                        output.get().start_over();
                    }
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["output"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
