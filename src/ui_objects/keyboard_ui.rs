use eframe::egui;

use crate::{
    core::soundprocessor::StaticSoundProcessorHandle,
    objects::keyboard::Keyboard,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct KeyboardUi {}

impl ObjectUi for KeyboardUi {
    type HandleType = StaticSoundProcessorHandle<Keyboard>;
    type StateType = NoUIState;

    fn ui(
        &self,
        keyboard: StaticSoundProcessorHandle<Keyboard>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        ObjectWindow::new_sound_processor(keyboard.id())
            .add_left_peg(keyboard.input.id(), "Input")
            .add_left_peg(&keyboard.key_frequency, "Note Frequency")
            .add_right_peg(keyboard.id(), "Output")
            .show(ui.ctx(), graph_tools, |ui, _graph_tools| {
                ui.label("Keyboard");
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
