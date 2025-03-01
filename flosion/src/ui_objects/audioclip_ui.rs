use eframe::egui;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

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

impl Stashable for AudioClipUiState {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.string(&self.name);
    }
}

impl UnstashableInplace for AudioClipUiState {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.string_inplace(&mut self.name)?;
        Ok(())
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
        ProcessorUi::new("AudioClip").show_with(
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
                        let dialog = rfd::FileDialog::new().add_filter(
                            "Audio files",
                            &["aiff", "ogg", "wav", "flac", "mp3", "m4a"],
                        );
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
