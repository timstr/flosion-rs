use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, ObjectId},
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::{validate_number_connection, validate_sound_connection},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

#[derive(Copy, Clone)]
pub(super) enum HotKeyAction {
    Activate(GraphId),
    Connect(GraphId, GraphId),
}

pub(super) struct PegHotKeys {
    mapping: HashMap<GraphId, (egui::Key, HotKeyAction)>,
}

impl PegHotKeys {
    fn new() -> PegHotKeys {
        PegHotKeys {
            mapping: HashMap::new(),
        }
    }

    pub(super) fn mapping(&self) -> &HashMap<GraphId, (egui::Key, HotKeyAction)> {
        &self.mapping
    }

    fn replace(&mut self, mapping: HashMap<GraphId, (egui::Key, HotKeyAction)>) {
        self.mapping = mapping;
    }

    pub(super) fn retain(&mut self, ids: &HashSet<GraphId>) {
        self.mapping.retain(|i, _| ids.contains(i));
    }

    pub(super) fn clear(&mut self) {
        self.mapping.clear();
    }

    pub(super) fn assign_hotkeys_to_pegs(&mut self, pegs_actions: &[(GraphId, HotKeyAction)]) {
        // TODO: arrange hotkeys such that their onscreen layout corresponds reasonably well to the keyboard layout
        let avail_keys: Vec<egui::Key> = vec![
            egui::Key::A,
            egui::Key::B,
            egui::Key::C,
            egui::Key::D,
            egui::Key::E,
            egui::Key::F,
            egui::Key::G,
            egui::Key::H,
            egui::Key::I,
            egui::Key::J,
            egui::Key::K,
            egui::Key::L,
            egui::Key::M,
            egui::Key::N,
            egui::Key::O,
            egui::Key::P,
            egui::Key::Q,
            egui::Key::R,
            egui::Key::S,
            egui::Key::T,
            egui::Key::U,
            egui::Key::V,
            egui::Key::W,
            egui::Key::X,
            egui::Key::Y,
            egui::Key::Z,
        ];
        let mut next_avail_key = avail_keys.iter();
        let mut mapping = HashMap::<GraphId, (egui::Key, HotKeyAction)>::new();
        for (p, a) in pegs_actions {
            if let Some(k) = next_avail_key.next() {
                self.mapping.insert(*p, (*k, *a));
            } else {
                break;
            }
        }
    }

    pub(super) fn peg_has_hotkey(&self, id: GraphId) -> Option<egui::Key> {
        self.mapping.get(&id).map(|x| x.0)
    }

    pub(super) fn update_peg_hotkeys_from_keyboard_focus(
        &mut self,
        topo: &SoundGraphTopology,
        keyboard_focus_state: &KeyboardFocusState,
    ) {
        let mut available_pegs: Vec<(GraphId, HotKeyAction)> = Vec::new();
        match *keyboard_focus_state {
            KeyboardFocusState::SoundProcessor(spid) => {
                let sp = topo.sound_processor(spid).unwrap();
                available_pegs.push((spid.into(), HotKeyAction::Activate(spid.into())));
                for si in sp.sound_inputs().iter().cloned() {
                    available_pegs.push((si.into(), HotKeyAction::Activate(si.into())));
                }
                for ni in sp.number_inputs().iter().cloned() {
                    available_pegs.push((ni.into(), HotKeyAction::Activate(ni.into())));
                }
                for ns in sp.number_sources().iter().cloned() {
                    available_pegs.push((ns.into(), HotKeyAction::Activate(ns.into())));
                }
            }
            KeyboardFocusState::NumberSource(nsid) => {
                let ns = topo.number_source(nsid).unwrap();
                available_pegs.push((nsid.into(), HotKeyAction::Activate(nsid.into())));
                for ni in ns.inputs().iter().cloned() {
                    available_pegs.push((ni.into(), HotKeyAction::Activate(ni.into())));
                }
            }
            KeyboardFocusState::SoundInput(siid) => {
                for spid in topo.sound_processors().keys().cloned() {
                    if validate_sound_connection(topo, siid, spid).is_ok() {
                        available_pegs
                            .push((spid.into(), HotKeyAction::Connect(siid.into(), spid.into())));
                    }
                }
            }
            KeyboardFocusState::SoundOutput(spid) => {
                for siid in topo.sound_inputs().keys().cloned() {
                    if validate_sound_connection(topo, siid, spid).is_ok() {
                        available_pegs
                            .push((siid.into(), HotKeyAction::Connect(spid.into(), siid.into())));
                    }
                }
            }
            KeyboardFocusState::NumberInput(niid) => {
                for nsid in topo.number_sources().keys().cloned() {
                    if validate_number_connection(topo, niid, nsid).is_ok() {
                        available_pegs
                            .push((nsid.into(), HotKeyAction::Connect(niid.into(), nsid.into())));
                    }
                }
            }
            KeyboardFocusState::NumberOutput(nsid) => {
                for niid in topo.number_inputs().keys().cloned() {
                    if validate_number_connection(topo, niid, nsid).is_ok() {
                        available_pegs
                            .push((niid.into(), HotKeyAction::Connect(nsid.into(), niid.into())));
                    }
                }
            }
        }
        // TODO: try to persist hotkeys of pegs that remain available
        self.clear();
        self.assign_hotkeys_to_pegs(&available_pegs);
    }
}

#[derive(Copy, Clone)]
pub(super) enum KeyboardFocusState {
    SoundProcessor(SoundProcessorId),
    NumberSource(NumberSourceId),
    SoundInput(SoundInputId),
    SoundOutput(SoundProcessorId),
    NumberInput(NumberInputId),
    NumberOutput(NumberSourceId),
}

impl KeyboardFocusState {
    pub(super) fn as_graph_id(&self) -> GraphId {
        match self {
            KeyboardFocusState::SoundProcessor(i) => (*i).into(),
            KeyboardFocusState::NumberSource(i) => (*i).into(),
            KeyboardFocusState::SoundInput(i) => (*i).into(),
            KeyboardFocusState::SoundOutput(i) => (*i).into(),
            KeyboardFocusState::NumberInput(i) => (*i).into(),
            KeyboardFocusState::NumberOutput(i) => (*i).into(),
        }
    }

    pub(super) fn object_has_keyboard_focus(&self, object: ObjectId) -> bool {
        match (object, self) {
            (ObjectId::Sound(spid1), KeyboardFocusState::SoundProcessor(spid2)) => spid1 == *spid2,
            (ObjectId::Number(nsid1), KeyboardFocusState::NumberSource(nsid2)) => nsid1 == *nsid2,
            _ => false,
        }
    }

    pub(super) fn peg_has_keyboard_focus(&self, id: GraphId) -> bool {
        match (id, self) {
            (GraphId::NumberInput(i1), KeyboardFocusState::NumberInput(i2)) => i1 == *i2,
            (GraphId::NumberSource(i1), KeyboardFocusState::NumberOutput(i2)) => i1 == *i2,
            (GraphId::SoundInput(i1), KeyboardFocusState::SoundInput(i2)) => i1 == *i2,
            (GraphId::SoundProcessor(i1), KeyboardFocusState::SoundOutput(i2)) => i1 == *i2,
            (_, _) => false,
        }
    }
}
