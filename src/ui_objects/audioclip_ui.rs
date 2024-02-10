use eframe::egui;

use crate::{
    core::{
        audiofileio::load_audio_file,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::audioclip::AudioClip,
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
        ProcessorUi::new(&audioclip, "AudioClip", data.color).show_with(
            ui,
            ctx,
            ui_state,
            sound_graph,
            |ui, _uistate, _sound_graph| {
                // TODO
                // - button to save to a file

                if ui.button("Load").clicked() {
                    let dialog =
                        rfd::FileDialog::new().add_filter("Audio files", &["wav", "flac", "m4a"]);
                    if let Some(path) = dialog.pick_file() {
                        println!("Loading audioclip from {}", path.display());
                        match load_audio_file(&path) {
                            Ok(buf) => audioclip.set_data(buf),
                            Err(e) => println!("Failed to load file: {}", e),
                        }
                    }
                }
            },
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["audioclip"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
