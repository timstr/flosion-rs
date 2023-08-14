use eframe::egui;

use crate::{
    core::soundprocessor::StaticSoundProcessorHandle,
    objects::keyboard::Keyboard,
    ui_core::{
        graph_ui_state::GraphUiState,
        object_ui::{ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct KeyboardUi {}

impl ObjectUi for KeyboardUi {
    type HandleType = StaticSoundProcessorHandle<Keyboard>;
    type StateType = ();

    fn ui(
        &self,
        keyboard: StaticSoundProcessorHandle<Keyboard>,
        ui_state: &mut GraphUiState,
        ui: &mut eframe::egui::Ui,
        data: ObjectUiData<()>,
    ) {
        ObjectWindow::new_sound_processor(keyboard.id(), "Keyboard", data.color)
            // .add_left_peg(keyboard.input.id(), "Input")
            // .add_left_peg(&keyboard.key_frequency, "Note Frequency")
            // .add_left_peg(&keyboard.key_time, "Note Time")
            // .add_right_peg(keyboard.id(), "Output")
            .show_with(ui.ctx(), ui_state, |ui, _ui_state| {
                let has_focus_id = egui::Id::new("keyboard_has_focus").with(keyboard.id());

                let had_focus = ui.memory_mut(|m| m.data.get_temp(has_focus_id).unwrap_or(false));

                let mut has_focus = had_focus;

                let label = if has_focus { "Stop" } else { "Play" };
                // TODO: fix the colour here
                let r = ui.toggle_value(&mut has_focus, label);

                if r.clicked_elsewhere() {
                    has_focus = false;
                }

                ui.memory_mut(|m| m.data.insert_temp(has_focus_id, has_focus));

                if !has_focus {
                    if had_focus {
                        keyboard.release_all_keys();
                    }
                    return;
                }

                let all_keys = [
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
                ];

                for e in ui.input(|i| i.events.clone()) {
                    if let egui::Event::Key {
                        key,
                        pressed,
                        repeat,
                        modifiers,
                    } = e
                    {
                        if repeat || modifiers.any() {
                            continue;
                        }
                        let Some(i) = all_keys.iter().position(|k| *k == key) else {
                            continue;
                        };
                        if pressed {
                            let f = 256.0_f32 * (2.0_f32).powf((i as f32) / 12.0_f32);
                            // let f = 128.0_f32 * ((i + 1) as f32); // heh
                            keyboard.start_key(i as u8, f);
                        } else {
                            keyboard.release_key(i as u8)
                        }
                    }
                }
            });
    }
}
