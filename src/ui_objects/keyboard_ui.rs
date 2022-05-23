use eframe::egui;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedStaticSoundProcessor},
    objects::keyboard::Keyboard,
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{
            NumberOutputWidget, ObjectUi, ObjectWindow, SoundInputWidget, SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct KeyboardUi {}

impl ObjectUi for KeyboardUi {
    type WrapperType = WrappedStaticSoundProcessor<Keyboard>;
    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedStaticSoundProcessor<Keyboard>,
        graph_state: &mut GraphUITools,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("Keyboard");
            ui.add(SoundInputWidget::new(
                object.input.id(),
                "Input",
                graph_state,
            ));
            ui.add(NumberOutputWidget::new(
                object.key_frequency.id(),
                "Note Frequency",
                graph_state,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_state));
            for (i, k) in [
                egui::Key::A, // C
                egui::Key::W, // C#
                egui::Key::S, // D
                egui::Key::E, // D#
                egui::Key::D, // E
                egui::Key::F, // F nice
                egui::Key::T, // F#
                egui::Key::G, // G nice
                egui::Key::Y, // G#
                egui::Key::H, // A
                egui::Key::U, // A#
                egui::Key::J, // B
                egui::Key::K, // C
                egui::Key::O, // C#
                egui::Key::L, // D
                egui::Key::P, // D#
            ]
            .iter()
            .enumerate()
            {
                if ui.input().key_down(*k) {
                    let f = 256.0_f32 * (2.0_f32).powf((i as f32) / 12.0_f32);
                    object.press_key(i as u16, f);
                }
                if ui.input().key_released(*k) {
                    object.release_key(i as u16);
                }
            }
        });
    }
}
