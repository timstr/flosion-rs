use eframe::egui;

use crate::{
    core::{audiofileio::load_audio_file, sound::soundprocessor::SoundProcessorWithId},
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
    type ObjectType = SoundProcessorWithId<AudioClip>;
    type StateType = AudioClipUiState;
    fn ui(
        &self,
        audioclip: &mut SoundProcessorWithId<AudioClip>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        state: &mut AudioClipUiState,
    ) {
        ProcessorUi::new(audioclip.id(), "AudioClip").show_with(
            audioclip,
            ui,
            ctx,
            graph_ui_state,
            |audioclip, ui, _uistate| {
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
        _handle: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<AudioClipUiState, ()> {
        // TODO: use args
        Ok(AudioClipUiState {
            name: "".to_string(),
        })
    }
}
