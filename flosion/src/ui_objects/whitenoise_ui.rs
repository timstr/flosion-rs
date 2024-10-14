use eframe::egui;

use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::whitenoise::WhiteNoise,
    ui_core::{
        arguments::ParsedArguments, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectui::SoundObjectUi,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl SoundObjectUi for WhiteNoiseUi {
    type ObjectType = SoundProcessorWithId<WhiteNoise>;
    type StateType = ();

    fn ui(
        &self,
        whitenoise: &mut SoundProcessorWithId<WhiteNoise>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
    ) {
        ProcessorUi::new(whitenoise.id(), "WhiteNoise").show(whitenoise, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["whitenoise"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::ObjectType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
