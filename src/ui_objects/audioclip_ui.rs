use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::audioclip::AudioClip,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct AudioClipUi {}

impl ObjectUi for AudioClipUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<AudioClip>;
    type StateType = ();
    fn ui(
        &self,
        audioclip: DynamicSoundProcessorHandle<AudioClip>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(audioclip.id(), "AudioClip", data.color).show(
            ui,
            ctx,
            ui_state,
            sound_graph,
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["audioclip"]
    }
}
