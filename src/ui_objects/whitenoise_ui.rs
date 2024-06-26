use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::whitenoise::WhiteNoise,
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
        ctx: &SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&whitenoise, "WhiteNoise", data.color).show(
            ui,
            ctx,
            ui_state,
            sound_graph,
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["whitenoise"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
