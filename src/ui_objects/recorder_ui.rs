use eframe::egui::{self, Button};

use crate::{
    core::{
        graphobject::{ObjectId, ObjectInitialization},
        soundprocessor::StaticSoundProcessorHandle,
    },
    objects::{audioclip::AudioClip, recorder::Recorder},
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow, SoundInputWidget, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct RecorderUi;

impl ObjectUi for RecorderUi {
    type WrapperType = StaticSoundProcessorHandle<Recorder>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &StaticSoundProcessorHandle<Recorder>,
        graph_tools: &mut GraphUIState,
        ui: &mut egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Recorder");
            ui.add(SoundInputWidget::new(
                object.input.id(),
                "Input",
                graph_tools,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
            let r = object.is_recording();
            let n = object.recording_length();
            let btn_str = if r {
                "Stop"
            } else if n > 0 {
                "Resume"
            } else {
                "Start"
            };
            if ui.add(Button::new(btn_str)).clicked() {
                if r {
                    object.stop_recording();
                } else {
                    object.start_recording();
                }
            }
            if n > 0 && !r {
                if ui.add(Button::new("Clear")).clicked() {
                    object.clear_recording();
                }
                if ui.add(Button::new("Create AudioClip")).clicked() {
                    let a = object.copy_audio();
                    graph_tools.make_change(move |graph| {
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
