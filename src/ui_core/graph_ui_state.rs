use std::any::{type_name, Any};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::rc::Rc;

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

pub struct LayoutState {
    pub rect: egui::Rect,
    pub layer: egui::LayerId,
}

impl LayoutState {
    pub fn center(&self) -> egui::Pos2 {
        self.rect.center()
    }
}

pub struct LayoutStateMap<T: Hash + Eq> {
    states: HashMap<T, LayoutState>,
}

impl<T: Hash + Eq> LayoutStateMap<T> {
    pub fn new() -> LayoutStateMap<T> {
        LayoutStateMap {
            states: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.states.clear();
    }

    fn states(&self) -> &HashMap<T, LayoutState> {
        &self.states
    }

    fn states_mut(&mut self) -> &mut HashMap<T, LayoutState> {
        &mut self.states
    }

    pub fn add(&mut self, id: T, rect: egui::Rect, layer: egui::LayerId) {
        let state = LayoutState { rect, layer };
        self.states.insert(id, state);
    }
}

pub struct GraphLayout {
    sound_inputs: LayoutStateMap<SoundInputId>,
    sound_outputs: LayoutStateMap<SoundProcessorId>,
    number_inputs: LayoutStateMap<NumberInputId>,
    number_outputs: LayoutStateMap<NumberSourceId>,
    objects: LayoutStateMap<ObjectId>,
}

impl GraphLayout {
    pub(super) fn new() -> GraphLayout {
        GraphLayout {
            sound_inputs: LayoutStateMap::new(),
            sound_outputs: LayoutStateMap::new(),
            number_inputs: LayoutStateMap::new(),
            number_outputs: LayoutStateMap::new(),
            objects: LayoutStateMap::new(),
        }
    }

    fn reset_pegs(&mut self) {
        self.sound_inputs.clear();
        self.sound_outputs.clear();
        self.number_inputs.clear();
        self.number_outputs.clear();
    }

    fn forget_object(&mut self, id: ObjectId) {
        self.objects.states_mut().remove(&id);
    }

    pub fn track_peg(&mut self, id: GraphId, rect: egui::Rect, layer: egui::LayerId) {
        match id {
            GraphId::NumberInput(id) => self.number_inputs.add(id, rect, layer),
            GraphId::NumberSource(id) => self.number_outputs.add(id, rect, layer),
            GraphId::SoundInput(id) => self.sound_inputs.add(id, rect, layer),
            GraphId::SoundProcessor(id) => self.sound_outputs.add(id, rect, layer),
        }
    }

    fn objects(&self) -> &LayoutStateMap<ObjectId> {
        &self.objects
    }

    fn objects_mut(&mut self) -> &mut LayoutStateMap<ObjectId> {
        &mut self.objects
    }

    pub fn track_object_location(&mut self, id: ObjectId, rect: egui::Rect, layer: egui::LayerId) {
        self.objects.add(id, rect, layer);
    }

    pub fn get_object_location(&self, id: ObjectId) -> Option<&LayoutState> {
        self.objects.states().get(&id)
    }

    pub(super) fn sound_inputs(&self) -> &HashMap<SoundInputId, LayoutState> {
        &self.sound_inputs.states
    }

    pub(super) fn sound_outputs(&self) -> &HashMap<SoundProcessorId, LayoutState> {
        &self.sound_outputs.states
    }

    pub(super) fn number_inputs(&self) -> &HashMap<NumberInputId, LayoutState> {
        &self.number_inputs.states
    }

    pub(super) fn number_outputs(&self) -> &HashMap<NumberSourceId, LayoutState> {
        &self.number_outputs.states
    }

    pub(super) fn find_peg_near(&self, position: egui::Pos2, ui: &egui::Ui) -> Option<GraphId> {
        let top_layer = match ui.memory().layer_id_at(position, ui.input().aim_radius()) {
            Some(a) => a,
            None => return None,
        };
        fn find<T: UniqueId>(
            peg_states: &LayoutStateMap<T>,
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
}

pub trait ObjectUiState: 'static {
    fn as_any(&self) -> &dyn Any;
    fn get_language_type_name(&self) -> &'static str;
}

impl<T: 'static> ObjectUiState for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }
}

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

pub struct GraphUIState {
    layout_state: GraphLayout,
    object_states: HashMap<ObjectId, Rc<RefCell<dyn ObjectUiState>>>,
    peg_being_dragged: Option<GraphId>,
    dropped_peg: Option<(GraphId, egui::Pos2)>,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph) -> ()>>,
    selection: HashSet<ObjectId>,
}

impl GraphUIState {
    pub(super) fn new() -> GraphUIState {
        GraphUIState {
            layout_state: GraphLayout::new(),
            object_states: HashMap::new(),
            peg_being_dragged: None,
            dropped_peg: None,
            pending_changes: Vec::new(),
            selection: HashSet::new(),
        }
    }

    pub(super) fn reset_pegs(&mut self) {
        self.layout_state.reset_pegs();
        self.dropped_peg = None;
    }

    pub(super) fn layout_state(&self) -> &GraphLayout {
        &self.layout_state
    }

    pub(super) fn layout_state_mut(&mut self) -> &mut GraphLayout {
        &mut self.layout_state
    }

    pub fn set_object_state(&mut self, id: ObjectId, state: Rc<RefCell<dyn ObjectUiState>>) {
        self.object_states.insert(id, state);
    }

    pub fn get_object_state(&mut self, id: ObjectId) -> Rc<RefCell<dyn ObjectUiState>> {
        Rc::clone(&self.object_states.get(&id).unwrap())
    }

    pub fn make_change<F: FnOnce(&mut SoundGraph) -> () + 'static>(&mut self, f: F) {
        self.pending_changes.push(Box::new(f));
    }

    pub(super) fn start_dragging(&mut self, graph_id: GraphId) {
        debug_assert!(self.peg_being_dragged.is_none());
        self.peg_being_dragged = Some(graph_id);
    }

    pub(super) fn stop_dragging(&mut self, graph_id: GraphId, location: egui::Pos2) {
        if self.peg_being_dragged.is_none() {
            return;
        }
        let drag_peg = self.peg_being_dragged.take().unwrap();
        debug_assert!(drag_peg == graph_id);
        debug_assert!(self.dropped_peg.is_none());
        self.dropped_peg = Some((drag_peg, location));
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

    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    pub fn select_object(&mut self, object_id: ObjectId) {
        self.selection.insert(object_id);
    }

    pub fn deselect_object(&mut self, object_id: ObjectId) {
        self.selection.remove(&object_id);
    }

    pub fn select_with_rect(&mut self, rect: egui::Rect, change: SelectionChange) {
        if let SelectionChange::Replace = change {
            self.clear_selection();
        }
        for (object_id, object_state) in self.layout_state.objects().states() {
            if rect.intersects(object_state.rect) {
                if let SelectionChange::Subtract = change {
                    self.selection.remove(object_id);
                } else {
                    self.selection.insert(*object_id);
                }
            }
        }
    }

    pub fn forget_selection(&mut self) {
        for id in &self.selection {
            self.layout_state.forget_object(*id);
            self.object_states.remove(id);
        }
        self.selection.clear();
    }

    pub fn selection(&self) -> &HashSet<ObjectId> {
        &self.selection
    }

    pub fn is_object_selected(&self, object_id: ObjectId) -> bool {
        self.selection.contains(&object_id)
    }

    pub fn move_selection(&mut self, delta: egui::Vec2) {
        let objects = self.layout_state.objects_mut();
        for s in &self.selection {
            let state = objects.states_mut().get_mut(s).unwrap();
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
