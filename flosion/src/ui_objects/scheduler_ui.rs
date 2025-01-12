use eframe::egui;

use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::scheduler::Scheduler,
    ui_core::{
        arguments::ParsedArguments, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct SchedulerUi {}

impl SoundObjectUi for SchedulerUi {
    type ObjectType = SoundProcessorWithId<Scheduler>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        scheduler: &mut SoundProcessorWithId<Scheduler>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new("Scheduler")
            .add_sound_input(&scheduler.sound_input, "Input")
            .show(scheduler, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["scheduler"]
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
