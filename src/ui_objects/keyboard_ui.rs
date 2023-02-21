use eframe::egui;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::StaticSoundProcessorHandle},
    objects::keyboard::Keyboard,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberOutputWidget, ObjectUi, ObjectWindow, SoundInputWidget,
            SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct KeyboardUi {}

impl ObjectUi for KeyboardUi {
    type HandleType = StaticSoundProcessorHandle<Keyboard>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        keyboard: StaticSoundProcessorHandle<Keyboard>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Keyboard");
            ui.add(SoundInputWidget::new(
                keyboard.input.id(),
                "Input",
                graph_tools,
            ));
            ui.add(NumberOutputWidget::new(
                &keyboard.key_frequency,
                "Note Frequency",
                graph_tools,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
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
                // TODO: ignore key repeat events
                if ui.input().key_down(*k) {
                    let f = 256.0_f32 * (2.0_f32).powf((i as f32) / 12.0_f32);
                    // let f = 128.0_f32 * ((i + 1) as f32); // heh
                    keyboard.start_key(i as u8, f);
                }
                if ui.input().key_released(*k) {
                    keyboard.release_key(i as u8);
                }
            }
        });
    }
}
