use chive::{Chivable, ChiveIn, ChiveOut};
use eframe::egui;

use crate::{
    core::{
        audiofileio::load_audio_file,
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::audioclip::AudioClip,
    ui_core::{
        arguments::ArgumentList, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
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

impl Chivable for AudioClipUiState {
    fn chive_in(&self, chive_in: &mut ChiveIn) {
        chive_in.string(&self.name);
    }

    fn chive_out(chive_out: &mut ChiveOut) -> Result<Self, ()> {
        Ok(AudioClipUiState {
            name: chive_out.string()?,
        })
    }
}

impl ObjectUi for AudioClipUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<AudioClip>;
    type StateType = AudioClipUiState;
    fn ui(
        &self,
        audioclip: DynamicSoundProcessorHandle<AudioClip>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        state: &mut AudioClipUiState,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&audioclip, "AudioClip").show_with(
            ui,
            ctx,
            graph_ui_state,
            sound_graph,
            |ui, _uistate, _sound_graph| {
                ui.vertical(|ui| {
                    if !state.name.is_empty() {
                        ui.add(egui::Label::new(
                            egui::RichText::new(&state.name)
                                .color(egui::Color32::BLACK)
                                .strong(),
                        ));
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
                                    state.name =
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

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: ObjectInitialization,
    ) -> Result<AudioClipUiState, ()> {
        // TODO: use init
        Ok(AudioClipUiState {
            name: "".to_string(),
        })
    }
}
