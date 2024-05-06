use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
    objects::keyboard::{KeyId, Keyboard},
    ui_core::{
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct KeyboardUi {}

impl ObjectUi for KeyboardUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Keyboard>;
    type StateType = ();

    fn ui(
        &self,
        keyboard: StaticSoundProcessorHandle<Keyboard>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&keyboard, "Keyboard", data.color)
            .add_sound_input(keyboard.input.id(), "input", sound_graph)
            .add_number_source(keyboard.key_frequency.id(), "keyfrequency")
            .show_with(
                ui,
                ctx,
                ui_state,
                sound_graph,
                |ui, _ui_state, _sound_graph| {
                    let has_focus_id = egui::Id::new("keyboard_has_focus").with(keyboard.id());

                    let had_focus =
                        ui.memory_mut(|m| m.data.get_temp(has_focus_id).unwrap_or(false));

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
                            physical_key: _,
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
                                keyboard.start_key(KeyId(i), f);
                            } else {
                                keyboard.release_key(KeyId(i));
                            }
                        }
                    }
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["keyboard"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
