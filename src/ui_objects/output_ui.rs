pub(crate) use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
    objects::output::Output,
    ui_core::{
        arguments::ParsedArguments, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct OutputUi {}

impl ObjectUi for OutputUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Output>;
    type StateType = ();
    fn ui(
        &self,
        output: StaticSoundProcessorHandle<Output>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&output, "Output")
            .add_sound_input(output.input.id(), "input", sound_graph)
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
                        output.start_over();
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

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
