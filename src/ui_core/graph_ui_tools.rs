use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, ObjectId},
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    soundgraph::SoundGraph,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

pub struct RectState {
    pub rect: egui::Rect,
    pub layer: egui::LayerId,
}

impl RectState {
    pub fn center(&self) -> egui::Pos2 {
        self.rect.center()
    }
}

pub struct RectStateMap<T: Hash + Eq> {
    states: HashMap<T, RectState>,
}

impl<T: Hash + Eq> RectStateMap<T> {
    pub fn new() -> RectStateMap<T> {
        RectStateMap {
            states: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.states.clear();
    }

    fn states(&self) -> &HashMap<T, RectState> {
        &self.states
    }

    fn states_mut(&mut self) -> &mut HashMap<T, RectState> {
        &mut self.states
    }

    pub fn add(&mut self, id: T, rect: egui::Rect, layer: egui::LayerId) {
        let state = RectState { rect, layer };
        self.states.insert(id, state);
    }
}

pub struct GraphUITools {
    sound_inputs: RectStateMap<SoundInputId>,
    sound_outputs: RectStateMap<SoundProcessorId>,
    number_inputs: RectStateMap<NumberInputId>,
    number_outputs: RectStateMap<NumberSourceId>,
    objects: RectStateMap<ObjectId>,
    peg_being_dragged: Option<GraphId>,
    dropped_peg: Option<(GraphId, egui::Pos2)>,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph) -> ()>>,
    selection: HashSet<ObjectId>,
}

impl GraphUITools {
    pub(super) fn new() -> GraphUITools {
        GraphUITools {
            sound_inputs: RectStateMap::new(),
            sound_outputs: RectStateMap::new(),
            number_inputs: RectStateMap::new(),
            number_outputs: RectStateMap::new(),
            objects: RectStateMap::new(),
            peg_being_dragged: None,
            dropped_peg: None,
            pending_changes: Vec::new(),
            selection: HashSet::new(),
        }
    }

    pub(super) fn reset_pegs(&mut self) {
        self.sound_inputs.clear();
        self.sound_outputs.clear();
        self.number_inputs.clear();
        self.number_outputs.clear();
        self.dropped_peg = None;
    }

    pub fn track_peg(&mut self, id: GraphId, rect: egui::Rect, layer: egui::LayerId) {
        match id {
            GraphId::NumberInput(id) => self.number_inputs.add(id, rect, layer),
            GraphId::NumberSource(id) => self.number_outputs.add(id, rect, layer),
            GraphId::SoundInput(id) => self.sound_inputs.add(id, rect, layer),
            GraphId::SoundProcessor(id) => self.sound_outputs.add(id, rect, layer),
        }
    }

    pub fn track_object(&mut self, id: ObjectId, rect: egui::Rect, layer: egui::LayerId) {
        self.objects.add(id, rect, layer);
    }

    pub fn get_object_state(&self, id: ObjectId) -> Option<&RectState> {
        self.objects.states().get(&id)
    }

    pub fn make_change<F: FnOnce(&mut SoundGraph) -> () + 'static>(&mut self, f: F) {
        self.pending_changes.push(Box::new(f));
    }

    pub(super) fn sound_inputs(&self) -> &HashMap<SoundInputId, RectState> {
        &self.sound_inputs.states
    }

    pub(super) fn sound_outputs(&self) -> &HashMap<SoundProcessorId, RectState> {
        &self.sound_outputs.states
    }

    pub(super) fn number_inputs(&self) -> &HashMap<NumberInputId, RectState> {
        &self.number_inputs.states
    }

    pub(super) fn number_outputs(&self) -> &HashMap<NumberSourceId, RectState> {
        &self.number_outputs.states
    }

    pub(super) fn start_dragging(&mut self, graph_id: GraphId) {
        debug_assert!(self.peg_being_dragged.is_none());
        self.peg_being_dragged = Some(graph_id);
    }

    pub(super) fn stop_dragging(&mut self, graph_id: GraphId, location: egui::Pos2) {
        debug_assert!(match self.peg_being_dragged {
            Some(i) => i == graph_id,
            None => false,
        });
        debug_assert!(self.dropped_peg.is_none());
        self.dropped_peg = Some((self.peg_being_dragged.take().unwrap(), location));
    }

    pub(super) fn peg_being_dragged(&self) -> Option<GraphId> {
        self.peg_being_dragged
    }

    pub(super) fn peg_was_dropped(&self) -> bool {
        self.dropped_peg.is_some()
    }

    pub(super) fn drop_location(&self) -> Option<egui::Pos2> {
        self.dropped_peg.map(|(_id, p)| p)
    }

    pub(super) fn dropped_peg_id(&self) -> Option<GraphId> {
        self.dropped_peg.map(|(id, _p)| id)
    }

    pub(super) fn find_peg_near(&self, position: egui::Pos2, ui: &egui::Ui) -> Option<GraphId> {
        let top_layer = match ui.memory().layer_id_at(position, ui.input().aim_radius()) {
            Some(a) => a,
            None => return None,
        };
        fn find<T: UniqueId>(
            peg_states: &RectStateMap<T>,
            layer: egui::LayerId,
            position: egui::Pos2,
        ) -> Option<T> {
            for (id, st) in peg_states.states() {
                if st.layer != layer {
                    continue;
                }
                if st.rect.contains(position) {
                    return Some(*id);
                }
            }
            None
        }

        if let Some(id) = find(&self.number_inputs, top_layer, position) {
            return Some(id.into());
        }
        if let Some(id) = find(&self.number_outputs, top_layer, position) {
            return Some(id.into());
        }
        if let Some(id) = find(&self.sound_inputs, top_layer, position) {
            return Some(id.into());
        }
        if let Some(id) = find(&self.sound_outputs, top_layer, position) {
            return Some(id.into());
        }
        None
    }

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn select_object(&mut self, object_id: ObjectId) {
        self.selection.insert(object_id);
    }

    pub fn deselect_object(&mut self, object_id: ObjectId) {
        self.selection.remove(&object_id);
    }

    pub fn select_with_rect(&mut self, rect: egui::Rect) {
        // TODO: allow shift/alt to add/remove objects from selection
        self.clear_selection();
        for (object_id, object_state) in self.objects.states() {
            if rect.contains_rect(object_state.rect) {
                self.selection.insert(*object_id);
            }
        }
    }

    pub fn selection(&self) -> &HashSet<ObjectId> {
        &self.selection
    }

    pub fn is_object_selected(&self, object_id: ObjectId) -> bool {
        self.selection.contains(&object_id)
    }

    pub fn move_selection(&mut self, delta: egui::Vec2) {
        for s in &self.selection {
            let state = self.objects.states_mut().get_mut(s).unwrap();
            state.rect = state.rect.translate(delta);
        }
    }

    pub(super) fn apply_pending_changes(&mut self, graph: &mut SoundGraph) {
        for f in self.pending_changes.drain(..) {
            f(graph);
        }
        debug_assert!(self.pending_changes.len() == 0);
    }
}
