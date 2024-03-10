use eframe::egui;
use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::{
        audiofileio::load_audio_file,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::audioclip::AudioClip,
    ui_core::{
        arguments::ArgumentList,
        graph_ui::ObjectUiState,
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

pub struct AudioClipUiState {
    name: String,
}

impl Default for AudioClipUiState {
    fn default() -> Self {
        Self {
            name: "".to_string(),
        }
    }
}

impl Serializable for AudioClipUiState {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.string(&self.name);
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(AudioClipUiState {
            name: deserializer.string()?,
        })
    }
}

impl ObjectUiState for AudioClipUiState {}

impl ObjectUi for AudioClipUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<AudioClip>;
    type StateType = AudioClipUiState;
    fn ui(
        &self,
        audioclip: DynamicSoundProcessorHandle<AudioClip>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<AudioClipUiState>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&audioclip, "AudioClip", data.color).show_with(
            ui,
            ctx,
            ui_state,
            sound_graph,
            |ui, _uistate, _sound_graph| {
                ui.vertical(|ui| {
                    if !data.state.name.is_empty() {
                        ui.label(&data.state.name);
                    }
                    // TODO
                    // - button to save to a file

                    if ui.button("Load").clicked() {
                        let dialog = rfd::FileDialog::new()
                            .add_filter("Audio files", &["wav", "flac", "m4a"]);
                        if let Some(path) = dialog.pick_file() {
                            println!("Loading audioclip from {}", path.display());
                            match load_audio_file(&path) {
                                Ok(buf) => {
                                    audioclip.set_data(buf);
                                    data.state.name =
                                        path.file_name().unwrap().to_str().unwrap().to_string();
                                }
                                Err(e) => println!("Failed to load file: {}", e),
                            }
                        }
                    }
                });
            },
        );
    }

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty().add(&AudioClip::ARG_PATH)
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["audioclip"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        (
            AudioClipUiState {
                name: "".to_string(),
            },
            Color::default(),
        )
    }
}
