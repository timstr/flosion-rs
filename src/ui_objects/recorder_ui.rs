use eframe::egui::{self, Button};

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
    objects::{audioclip::AudioClip, recorder::Recorder},
    ui_core::{
        arguments::ParsedArguments, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectui::SoundObjectUi,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct RecorderUi;

impl SoundObjectUi for RecorderUi {
    type HandleType = StaticSoundProcessorHandle<Recorder>;
    type StateType = ();

    fn ui(
        &self,
        recorder: StaticSoundProcessorHandle<Recorder>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&recorder, "Recorder")
            .add_sound_input(recorder.input.id(), "Input", sound_graph)
            .show_with(
                ui,
                ctx,
                graph_ui_state,
                sound_graph,
                |ui, _ui_state, sound_graph| {
                    let r = recorder.is_recording();
                    let n = recorder.recording_length();
                    let btn_str = if r {
                        "Stop"
                    } else if n > 0 {
                        "Resume"
                    } else {
                        "Start"
                    };
                    if ui.add(Button::new(btn_str)).clicked() {
                        if r {
                            recorder.stop_recording();
                        } else {
                            recorder.start_recording();
                        }
                    }
                    if n > 0 && !r {
                        if ui.add(Button::new("Clear")).clicked() {
                            recorder.clear_recording();
                        }
                        if ui.add(Button::new("Create AudioClip")).clicked() {
                            let a = recorder.copy_audio();
                            let ac = sound_graph.add_dynamic_sound_processor::<AudioClip>(
                                &ParsedArguments::new_empty(),
                            );
                            // TODO: move the audio clip nearby
                            match ac {
                                Ok(ac) => ac.set_data(a),
                                Err(_) => println!("Recorder failed to create an AudioClip"),
                            }
                        }
                    }
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["recorder"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
