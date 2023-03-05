use eframe::egui::{self, Button};

use crate::{
    core::{graphobject::ObjectInitialization, soundprocessor::StaticSoundProcessorHandle},
    objects::{audioclip::AudioClip, recorder::Recorder},
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct RecorderUi;

impl ObjectUi for RecorderUi {
    type HandleType = StaticSoundProcessorHandle<Recorder>;
    type StateType = NoUIState;

    fn ui(
        &self,
        recorder: StaticSoundProcessorHandle<Recorder>,
        graph_tools: &mut GraphUIState,
        ui: &mut egui::Ui,
        data: ObjectUiData<NoUIState>,
    ) {
        ObjectWindow::new_sound_processor(recorder.id(), "Recorder", data.color)
            .add_left_peg(recorder.input.id(), "Input")
            .add_right_peg(recorder.id(), "Output")
            .show_with(ui.ctx(), graph_tools, |ui, graph_tools| {
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
                        graph_tools.make_change(move |graph, _| {
                            let ac = graph.add_dynamic_sound_processor::<AudioClip>(
                                ObjectInitialization::Default,
                            );
                            match ac {
                                Ok(ac) => ac.set_data(a),
                                Err(_) => println!("Recorder failed to create an AudioClip"),
                            }
                        });
                    }
                }
            });
    }
}
