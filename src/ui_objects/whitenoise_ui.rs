use eframe::egui;

use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::whitenoise::WhiteNoise,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WhiteNoise>;
    type StateType = ();

    fn ui(
        &self,
        whitenoise: DynamicSoundProcessorHandle<WhiteNoise>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        ProcessorUi::new(whitenoise.id(), "WhiteNoise", data.color)
            // .add_right_peg(whitenoise.id(), "Output")
            .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["whitenoise"]
    }
}
