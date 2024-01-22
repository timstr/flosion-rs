use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
    objects::output::Output,
    ui_core::{
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
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
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&output, "Output", data.color)
            .add_sound_input(output.input.id(), "input", sound_graph)
            .show_with(
                ui,
                ctx,
                ui_state,
                sound_graph,
                |ui, _ui_state, _sound_graph| {
                    if ui.add(egui::Button::new("Reset").wrap(false)).clicked() {
                        output.reset();
                    }
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["output"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
