use eframe::egui;

use crate::core::sound::{
    soundgraphid::SoundGraphId, soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId, soundprocessor::SoundProcessorId,
};

use super::{soundnumberinputui::SoundNumberInputFocus, temporallayout::TemporalLayout};

pub(super) enum KeyboardFocusState {
    AroundSoundProcessor(SoundProcessorId),
    AroundSoundInput(SoundInputId),
    AroundSoundNumberInput(SoundNumberInputId),
    InsideSoundNumberInput(SoundNumberInputId, SoundNumberInputFocus),
}

impl KeyboardFocusState {
    pub(super) fn graph_id(&self) -> SoundGraphId {
        match self {
            KeyboardFocusState::AroundSoundProcessor(spid) => (*spid).into(),
            KeyboardFocusState::AroundSoundInput(siid) => (*siid).into(),
            KeyboardFocusState::AroundSoundNumberInput(niid) => (*niid).into(),
            KeyboardFocusState::InsideSoundNumberInput(niid, _) => (*niid).into(),
        }
    }

    pub(super) fn item_has_keyboard_focus(&self, item: SoundGraphId) -> bool {
        self.graph_id() == item
    }

    pub(super) fn sound_number_input_focus(
        &mut self,
        id: SoundNumberInputId,
    ) -> Option<&mut SoundNumberInputFocus> {
        match self {
            KeyboardFocusState::InsideSoundNumberInput(snid, focus) => {
                if *snid == id {
                    Some(focus)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_single_keyboard_event(
        &mut self,
        topology: &SoundGraphTopology,
        temporal_layout: &TemporalLayout,
        input: &mut egui::InputState,
    ) -> bool {
        if let KeyboardFocusState::InsideSoundNumberInput(niid, _) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundSoundNumberInput(*niid);
                return true;
            }
        } else {
            let graph_id = self.graph_id();
            let root_spid = temporal_layout.find_root_processor(graph_id, topology);

            let items = temporal_layout.get_stack_items(root_spid, topology);

            let mut index = items.iter().position(|i| *i == graph_id).unwrap();
            let mut did_anything = false;

            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                index = index.saturating_sub(1);
                did_anything = true;
            }

            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                index = (index + 1).min(items.len() - 1);
                did_anything = true;
            }

            if did_anything {
                let new_item = items[index];
                *self = match new_item {
                    SoundGraphId::SoundInput(siid) => KeyboardFocusState::AroundSoundInput(siid),
                    SoundGraphId::SoundProcessor(spid) => {
                        KeyboardFocusState::AroundSoundProcessor(spid)
                    }
                    SoundGraphId::SoundNumberInput(nsid) => {
                        KeyboardFocusState::AroundSoundNumberInput(nsid)
                    }
                    SoundGraphId::SoundNumberSource(_) => panic!(),
                };
                return true;
            }
        }

        if let KeyboardFocusState::AroundSoundNumberInput(niid) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                *self =
                    KeyboardFocusState::InsideSoundNumberInput(*niid, SoundNumberInputFocus::new());
                return true;
            }
        }

        if let KeyboardFocusState::AroundSoundInput(siid) = self {
            if let Some(target_spid) = topology.sound_input(*siid).unwrap().target() {
                if temporal_layout.is_top_level(target_spid.into())
                    && input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                {
                    *self = KeyboardFocusState::AroundSoundProcessor(target_spid);
                    return true;
                }
            }
        }

        false
    }

    pub(super) fn handle_move_keyboard_focus(
        &mut self,
        ui: &egui::Ui,
        topology: &SoundGraphTopology,
        temporal_layout: &TemporalLayout,
    ) {
        ui.input_mut(|i| {
            //  preemptively avoid some unnecessary computation
            if !(i.key_pressed(egui::Key::ArrowUp)
                || i.key_pressed(egui::Key::ArrowDown)
                || i.key_pressed(egui::Key::Enter)
                || i.key_pressed(egui::Key::Escape))
            {
                return;
            }
            while self.handle_single_keyboard_event(topology, temporal_layout, i) {}
        });
    }
}
