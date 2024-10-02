use eframe::egui;

use crate::{
    core::{
        audiofileio::load_audio_file,
        sound::{soundgraph::SoundGraph, soundprocessor::WhateverSoundProcessorHandle},
    },
    objects::audioclip::AudioClip,
    ui_core::{
        arguments::{ArgumentList, ParsedArguments},
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi,
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

impl SoundObjectUi for AudioClipUi {
    type HandleType = WhateverSoundProcessorHandle<AudioClip>;
    type StateType = AudioClipUiState;
    fn ui(
        &self,
        audioclip: WhateverSoundProcessorHandle<AudioClip>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        state: &mut AudioClipUiState,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(audioclip.id(), "AudioClip").show_with(
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
                                    audioclip.get_mut().set_data(buf);
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
        _args: &ParsedArguments,
    ) -> Result<AudioClipUiState, ()> {
        // TODO: use args
        Ok(AudioClipUiState {
            name: "".to_string(),
        })
    }
}
