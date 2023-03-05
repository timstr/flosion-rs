use std::{collections::HashSet, time::Duration};

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, ObjectId},
    numberinput::NumberInputOwner,
    numbersource::NumberSourceOwner,
    soundgraph::SoundGraph,
    soundgraphtopology::SoundGraphTopology,
    uniqueid::UniqueId,
};

use super::{
    diagnostics::{AllDiagnostics, Diagnostic, DiagnosticRelevance},
    hotkeys::{HotKeyAction, KeyboardFocusState, PegHotKeys},
    layout_state::GraphLayout,
};

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

enum UiMode {
    Passive,
    DraggingPeg(GraphId),
    DroppingPeg((GraphId, egui::Pos2)),
    UsingKeyboardNav((KeyboardFocusState, PegHotKeys)),
    Selecting(HashSet<ObjectId>),
}

pub struct GraphUIState {
    layout_state: GraphLayout,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph, &mut GraphUIState) -> ()>>,
    mode: UiMode,
    diagnostics: AllDiagnostics,
}

impl GraphUIState {
    pub(super) fn new() -> GraphUIState {
        GraphUIState {
            layout_state: GraphLayout::new(),
            pending_changes: Vec::new(),
            mode: UiMode::Passive,
            diagnostics: AllDiagnostics::new(),
        }
    }

    pub(super) fn reset_pegs(&mut self) {
        self.layout_state.reset_pegs();
    }

    pub(super) fn layout_state(&self) -> &GraphLayout {
        &self.layout_state
    }

    pub(super) fn layout_state_mut(&mut self) -> &mut GraphLayout {
        &mut self.layout_state
    }

    pub fn make_change<F: FnOnce(&mut SoundGraph, &mut GraphUIState) -> () + 'static>(
        &mut self,
        f: F,
    ) {
        self.pending_changes.push(Box::new(f));
    }

    pub(super) fn start_dragging(&mut self, graph_id: GraphId) {
        self.mode = UiMode::DraggingPeg(graph_id);
    }

    pub(super) fn stop_dragging(&mut self, location: Option<egui::Pos2>) {
        if let UiMode::DraggingPeg(i) = &self.mode {
            self.mode = match location {
                Some(l) => UiMode::DroppingPeg((*i, l)),
                None => UiMode::Passive,
            };
        }
    }

    pub(super) fn peg_being_dragged(&self) -> Option<GraphId> {
        match self.mode {
            UiMode::DraggingPeg(id) => Some(id),
            _ => None,
        }
    }

    pub(super) fn take_peg_being_dropped(&mut self) -> Option<(GraphId, egui::Pos2)> {
        match self.mode {
            UiMode::DroppingPeg(p) => {
                self.mode = UiMode::Passive;
                Some(p)
            }
            _ => None,
        }
    }

    pub fn clear_selection(&mut self) {
        match self.mode {
            UiMode::Selecting(_) => self.mode = UiMode::Passive,
            _ => (),
        }
    }

    pub fn set_selection(&mut self, object_ids: HashSet<ObjectId>) {
        self.mode = UiMode::Selecting(object_ids);
    }

    pub fn select_object(&mut self, object_id: ObjectId) {
        match &mut self.mode {
            UiMode::Selecting(s) => {
                s.insert(object_id);
            }
            _ => {
                let mut s = HashSet::new();
                s.insert(object_id);
                self.mode = UiMode::Selecting(s);
            }
        }
    }

    pub fn deselect_object(&mut self, object_id: ObjectId) {
        match &mut self.mode {
            UiMode::Selecting(s) => {
                s.remove(&object_id);
                if s.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            _ => (),
        }
    }

    pub fn select_with_rect(&mut self, rect: egui::Rect, change: SelectionChange) {
        let mut selection = match &mut self.mode {
            UiMode::Selecting(s) => {
                let mut ss = HashSet::new();
                std::mem::swap(s, &mut ss);
                self.mode = UiMode::Passive;
                ss
            }
            _ => HashSet::new(),
        };

        if let SelectionChange::Replace = change {
            selection.clear();
        }
        for (object_id, object_state) in self.layout_state.objects() {
            if rect.intersects(object_state.rect) {
                if let SelectionChange::Subtract = change {
                    selection.remove(object_id);
                } else {
                    selection.insert(*object_id);
                }
            }
        }

        if selection.len() > 0 {
            self.mode = UiMode::Selecting(selection)
        } else {
            self.mode = UiMode::Passive;
        }
    }

    pub(super) fn cleanup(&mut self, remaining_ids: &HashSet<GraphId>) {
        self.layout_state.retain(remaining_ids);

        match &mut self.mode {
            UiMode::Selecting(s) => {
                s.retain(|id| remaining_ids.contains(&(*id).into()));
                if s.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Passive => (),
            UiMode::DraggingPeg(id) => {
                if !remaining_ids.contains(id) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::DroppingPeg((id, _)) => {
                if !remaining_ids.contains(id) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::UsingKeyboardNav((kbd_focus, hotkeys)) => {
                if !remaining_ids.contains(&kbd_focus.as_graph_id()) {
                    self.mode = UiMode::Passive;
                } else {
                    hotkeys.retain(&remaining_ids);
                }
            }
        }

        self.diagnostics.age_out(Duration::from_secs(1));
    }

    pub fn selection(&self) -> HashSet<ObjectId> {
        match &self.mode {
            UiMode::Selecting(s) => s.clone(),
            _ => HashSet::new(),
        }
    }

    pub fn is_object_selected(&self, object_id: ObjectId) -> bool {
        match &self.mode {
            UiMode::Selecting(s) => s.contains(&object_id),
            _ => false,
        }
    }

    pub fn move_selection(&mut self, delta: egui::Vec2) {
        let objects = self.layout_state.objects_mut();
        match &self.mode {
            UiMode::Selecting(selection) => {
                for s in selection {
                    let state = objects.get_mut(s).unwrap();
                    state.rect = state.rect.translate(delta);
                }
            }
            _ => (),
        }
    }

    pub fn object_has_keyboard_focus(&self, object_id: ObjectId) -> bool {
        match &self.mode {
            UiMode::UsingKeyboardNav(k) => k.0.object_has_keyboard_focus(object_id),
            _ => false,
        }
    }

    pub fn peg_has_keyboard_focus(&self, graph_id: GraphId) -> bool {
        match &self.mode {
            UiMode::UsingKeyboardNav(k) => k.0.peg_has_keyboard_focus(graph_id),
            _ => false,
        }
    }

    pub fn peg_has_hotkey(&self, graph_id: GraphId) -> Option<egui::Key> {
        match &self.mode {
            UiMode::UsingKeyboardNav(k) => k.1.peg_has_hotkey(graph_id),
            _ => None,
        }
    }

    pub fn issue_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push_diagnostic(diagnostic);
    }

    pub fn graph_item_has_warning(&self, graph_id: GraphId) -> Option<DiagnosticRelevance> {
        self.diagnostics.graph_item_has_warning(graph_id)
    }

    fn update_keyboard_focus_from_selection(&mut self) {
        // TODO: press key when one object is selected to enter focus
        // and give this mode a better name
        // self.keyboard_focus_state = KeyboardFocusState::Nothing;
        // if self.selection.len() != 1 {
        //     self.update_peg_hotkeys_from_keyboard_focus();
        //     return;
        // }
        // let selected_object = *self.selection.iter().next().unwrap();
        // self.keyboard_focus_state = match selected_object {
        //     ObjectId::Sound(spid) => KeyboardFocusState::SoundProcessor(spid),
        //     ObjectId::Number(nsid) => KeyboardFocusState::NumberSource(nsid),
        // };
        // self.update_peg_hotkeys_from_keyboard_focus();
    }

    pub(super) fn activate_hotkey(&mut self, key: egui::Key, topo: &SoundGraphTopology) -> bool {
        let (keyboard_focus, peg_hotkeys) = match &mut self.mode {
            UiMode::UsingKeyboardNav(p) => p,
            _ => return false,
        };

        let action = peg_hotkeys
            .mapping()
            .iter()
            .find_map(|(_, (k, a))| if *k == key { Some(*a) } else { None });
        let action = match action {
            Some(a) => a,
            None => return false,
        };
        match action {
            HotKeyAction::Activate(gid) => {
                *keyboard_focus = match gid {
                    GraphId::SoundInput(siid) => KeyboardFocusState::SoundInput(siid),
                    GraphId::SoundProcessor(spid) => KeyboardFocusState::SoundOutput(spid),
                    GraphId::NumberInput(niid) => KeyboardFocusState::NumberInput(niid),
                    GraphId::NumberSource(nsid) => KeyboardFocusState::NumberOutput(nsid),
                };
            }
            HotKeyAction::Connect(gid1, gid2) => {
                match (gid1, gid2) {
                    (GraphId::NumberInput(niid), GraphId::NumberSource(nsid)) => {
                        self.pending_changes.push(Box::new(move |g, _| {
                            g.disconnect_number_input(niid).unwrap();
                            g.connect_number_input(niid, nsid).unwrap();
                        }));
                        *keyboard_focus = KeyboardFocusState::NumberSource(nsid);
                    }
                    (GraphId::NumberSource(nsid), GraphId::NumberInput(niid)) => {
                        self.pending_changes.push(Box::new(move |g, _| {
                            g.disconnect_number_input(niid).unwrap();
                            g.connect_number_input(niid, nsid).unwrap();
                        }));
                        let owner = topo.number_input(niid).unwrap().owner();
                        *keyboard_focus = match owner {
                            NumberInputOwner::SoundProcessor(i) => {
                                KeyboardFocusState::SoundProcessor(i)
                            }
                            NumberInputOwner::NumberSource(i) => {
                                KeyboardFocusState::NumberSource(i)
                            }
                        };
                    }
                    (GraphId::SoundInput(siid), GraphId::SoundProcessor(spid)) => {
                        self.pending_changes.push(Box::new(move |g, _| {
                            g.disconnect_sound_input(siid).unwrap();
                            g.connect_sound_input(siid, spid).unwrap();
                        }));
                        *keyboard_focus = KeyboardFocusState::SoundProcessor(spid);
                    }
                    (GraphId::SoundProcessor(spid), GraphId::SoundInput(siid)) => {
                        self.pending_changes.push(Box::new(move |g, _| {
                            g.disconnect_sound_input(siid).unwrap();
                            g.connect_sound_input(siid, spid).unwrap();
                        }));
                        let owner = topo.sound_input(siid).unwrap().owner();
                        *keyboard_focus = KeyboardFocusState::SoundProcessor(owner);
                    }
                    (_, _) => panic!(),
                };
            }
        }
        peg_hotkeys.update_peg_hotkeys_from_keyboard_focus(topo, &keyboard_focus);
        true
    }

    pub(super) fn cancel_hotkey(&mut self, topo: &SoundGraphTopology) -> bool {
        let (keyboard_focus, peg_hotkeys) = match &mut self.mode {
            UiMode::UsingKeyboardNav(p) => p,
            _ => return false,
        };

        match keyboard_focus {
            KeyboardFocusState::SoundInput(siid) => {
                let o = topo.sound_input(*siid).unwrap().owner();
                *keyboard_focus = KeyboardFocusState::SoundProcessor(o);
            }
            KeyboardFocusState::SoundOutput(spid) => {
                *keyboard_focus = KeyboardFocusState::SoundProcessor(*spid);
            }
            KeyboardFocusState::NumberInput(niid) => {
                let o = topo.number_input(*niid).unwrap().owner();
                *keyboard_focus = match o {
                    NumberInputOwner::SoundProcessor(i) => KeyboardFocusState::SoundProcessor(i),
                    NumberInputOwner::NumberSource(i) => KeyboardFocusState::NumberSource(i),
                };
            }
            KeyboardFocusState::NumberOutput(nsid) => {
                let o = topo.number_source(*nsid).unwrap().owner();
                *keyboard_focus = match o {
                    NumberSourceOwner::Nothing => panic!(),
                    NumberSourceOwner::SoundProcessor(i) => KeyboardFocusState::SoundProcessor(i),
                    NumberSourceOwner::SoundInput(i) => {
                        KeyboardFocusState::SoundProcessor(topo.sound_input(i).unwrap().owner())
                    }
                };
            }
            KeyboardFocusState::SoundProcessor(_) => {
                self.mode = UiMode::Passive;
                return true;
            }
            KeyboardFocusState::NumberSource(_) => {
                self.mode = UiMode::Passive;
                return true;
            }
        };
        peg_hotkeys.update_peg_hotkeys_from_keyboard_focus(topo, keyboard_focus);
        true
    }

    pub(super) fn apply_pending_changes(&mut self, graph: &mut SoundGraph) {
        let mut pending_changes = Vec::new();
        std::mem::swap(&mut self.pending_changes, &mut pending_changes);
        for f in pending_changes {
            f(graph, self);
        }
        debug_assert!(self.pending_changes.is_empty());
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        let mut good = true;
        for i in self.layout_state.objects().keys() {
            match i {
                ObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(i) {
                        println!(
                            "An object position exists for a non-existent sound processor {}",
                            i.value()
                        );
                        good = false;
                    }
                }
                ObjectId::Number(i) => {
                    if !topo.number_sources().contains_key(i) {
                        println!(
                            "An object position exists for a non-existent number source {}",
                            i.value()
                        );
                        good = false;
                    }
                }
            }
        }
        for i in self.layout_state.sound_outputs().keys() {
            if !topo.sound_processors().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent sound output {}",
                    i.value()
                );
                good = false;
            }
        }
        for i in self.layout_state.sound_inputs().keys() {
            if !topo.sound_inputs().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent sound input {}",
                    i.value()
                );
                good = false;
            }
        }
        for i in self.layout_state.number_outputs().keys() {
            if !topo.number_sources().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent number output {}",
                    i.value()
                );
                good = false;
            }
        }
        for i in self.layout_state.number_inputs().keys() {
            if !topo.number_inputs().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent number output {}",
                    i.value()
                );
                good = false;
            }
        }
        good
    }

    pub(super) fn select_all(&mut self, topo: &SoundGraphTopology) {
        let mut ids: HashSet<ObjectId> = HashSet::new();
        {
            for i in topo.sound_processors().keys() {
                ids.insert(i.into());
            }
            for (i, ns) in topo.number_sources() {
                if ns.owner() == NumberSourceOwner::Nothing {
                    ids.insert(i.into());
                }
            }
        }
        self.set_selection(ids);
    }

    pub(super) fn select_none(&mut self) {
        if let UiMode::Selecting(_) = self.mode {
            self.mode = UiMode::Passive;
        }
    }
}
