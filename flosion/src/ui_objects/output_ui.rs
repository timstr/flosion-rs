pub(crate) use eframe::egui;

use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::output::Output,
    ui_core::{
        arguments::ParsedArguments, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct OutputUi {}

impl SoundObjectUi for OutputUi {
    type ObjectType = SoundProcessorWithId<Output>;
    type StateType = NoObjectUiState;
    fn ui(
        &self,
        output: &mut SoundProcessorWithId<Output>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new(output.id(), "Output")
            .add_sound_input(&output.input, "input")
            .show_with(output, ui, ctx, graph_ui_state, |output, ui, _ui_state| {
                if ui
                    .add(egui::Button::new("Start over").wrap_mode(egui::TextWrapMode::Extend))
                    .clicked()
                {
                    output.start_over();
                }
            });
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["output"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<NoObjectUiState, ()> {
        Ok(NoObjectUiState)
    }
}
