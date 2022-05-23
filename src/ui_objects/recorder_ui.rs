use eframe::egui::{self, Button};
use futures::executor::block_on;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedStaticSoundProcessor},
    objects::{audioclip::AudioClip, recorder::Recorder},
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{ObjectUi, ObjectWindow, SoundInputWidget, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct RecorderUi;

impl ObjectUi for RecorderUi {
    type WrapperType = WrappedStaticSoundProcessor<Recorder>;

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedStaticSoundProcessor<Recorder>,
        graph_state: &mut GraphUITools,
        ui: &mut egui::Ui,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("Recorder");
            ui.add(SoundInputWidget::new(
                object.input.id(),
                "Input",
                graph_state,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_state));
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
                    graph_state.make_change(move |graph| {
                        let ac = block_on(graph.add_dynamic_sound_processor::<AudioClip>());
                        ac.instance().set_data(a);
                    });
                }
            }
        });
    }
}
