use std::collections::HashSet;

use eframe::egui;

use crate::core::{
    graphobject::{ObjectId, SoundGraphId},
    soundgraph::SoundGraph,
    soundgraphtopology::SoundGraphTopology,
    uniqueid::UniqueId,
};

use super::{
    hotkeys::KeyboardFocusState, object_positions::ObjectPositions, ui_context::TemporalLayout,
};

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

enum UiMode {
    Passive,
    UsingKeyboardNav(KeyboardFocusState),
    Selecting(HashSet<ObjectId>),
}

pub struct GraphUIState {
    object_positions: ObjectPositions,
    temporal_layout: TemporalLayout,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph, &mut GraphUIState) -> ()>>,
    mode: UiMode,
}

impl GraphUIState {
    pub(super) fn new() -> GraphUIState {
        GraphUIState {
            object_positions: ObjectPositions::new(),
            temporal_layout: TemporalLayout::new(),
            pending_changes: Vec::new(),
            mode: UiMode::Passive,
        }
    }

    pub(super) fn object_positions(&self) -> &ObjectPositions {
        &self.object_positions
    }

    pub(super) fn object_positions_mut(&mut self) -> &mut ObjectPositions {
        &mut self.object_positions
    }

    pub(super) fn temporal_layout(&self) -> &TemporalLayout {
        &self.temporal_layout
    }

    pub(super) fn temporal_layout_mut(&mut self) -> &mut TemporalLayout {
        &mut self.temporal_layout
    }

    pub fn make_change<F: FnOnce(&mut SoundGraph, &mut GraphUIState) -> () + 'static>(
        &mut self,
        f: F,
    ) {
        self.pending_changes.push(Box::new(f));
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
        for (object_id, object_state) in self.object_positions.objects() {
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

    pub(super) fn cleanup(
        &mut self,
        remaining_ids: &HashSet<SoundGraphId>,
        topo: &SoundGraphTopology,
    ) {
        self.object_positions.retain(remaining_ids);
        self.temporal_layout.retain(remaining_ids);

        match &mut self.mode {
            UiMode::Selecting(s) => {
                s.retain(|id| remaining_ids.contains(&(*id).into()));
                if s.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Passive => (),
            UiMode::UsingKeyboardNav(kbd_focus) => {
                if !remaining_ids.contains(&kbd_focus.as_graph_id()) {
                    self.mode = UiMode::Passive;
                }
            }
        }

        // TODO: do this conservatively, e.g. when the topology changes
        self.temporal_layout.regenerate(topo);
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

    pub fn move_selection(&mut self, delta: egui::Vec2, excluded: Option<ObjectId>) {
        let objects = self.object_positions.objects_mut();
        match &self.mode {
            UiMode::Selecting(selection) => {
                for s in selection {
                    if Some(*s) != excluded {
                        let state = objects.get_mut(s).unwrap();
                        state.rect = state.rect.translate(delta);
                    }
                }
            }
            _ => (),
        }
    }

    pub fn object_has_keyboard_focus(&self, object_id: ObjectId) -> bool {
        match &self.mode {
            UiMode::UsingKeyboardNav(k) => k.object_has_keyboard_focus(object_id),
            _ => false,
        }
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
        for i in self.object_positions.objects().keys() {
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
        }
        self.set_selection(ids);
    }

    pub(super) fn select_none(&mut self) {
        if let UiMode::Selecting(_) = self.mode {
            self.mode = UiMode::Passive;
        }
    }

    pub(super) fn create_state_for(&mut self, object_id: ObjectId, topo: &SoundGraphTopology) {
        self.object_positions.create_state_for(object_id);
    }
}
