use std::{
    any::{type_name, Any},
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::Hash,
    rc::Rc,
    sync::Arc,
};

use eframe::egui;
use parking_lot::RwLock;

use crate::core::{
    graphobject::{GraphId, ObjectId},
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    soundgraph::SoundGraph,
    soundgraphtopology::SoundGraphTopology,
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

#[derive(Copy, Clone)]
enum HotKeyAction {
    Activate(GraphId),
    Connect(GraphId, GraphId),
}

struct PegHotKeys {
    mapping: HashMap<GraphId, (egui::Key, HotKeyAction)>,
}

impl PegHotKeys {
    fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    fn replace(&mut self, mapping: HashMap<GraphId, (egui::Key, HotKeyAction)>) {
        self.mapping = mapping;
    }

    fn clear(&mut self) {
        self.mapping.clear();
    }
}

#[derive(Copy, Clone)]
pub enum KeyboardFocusState {
    Nothing,
    SoundProcessor(SoundProcessorId),
    NumberSource(NumberSourceId),
    SoundInput(SoundInputId),
    SoundOutput(SoundProcessorId),
    NumberInput(NumberInputId),
    NumberOutput(NumberSourceId),
}

pub struct GraphUIState {
    // TODO: move some things here into an enum to make mutual exclusion clearer
    layout_state: GraphLayout,
    object_states: HashMap<ObjectId, Rc<RefCell<dyn ObjectUiState>>>,
    peg_being_dragged: Option<GraphId>,
    dropped_peg: Option<(GraphId, egui::Pos2)>,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph) -> ()>>,
    keyboard_focus_state: KeyboardFocusState,
    peg_hotkeys: PegHotKeys,
    selection: HashSet<ObjectId>,
    graph_topology: Arc<RwLock<SoundGraphTopology>>,
}

impl GraphUIState {
    pub(super) fn new(topology: Arc<RwLock<SoundGraphTopology>>) -> GraphUIState {
        GraphUIState {
            layout_state: GraphLayout::new(),
            object_states: HashMap::new(),
            peg_being_dragged: None,
            dropped_peg: None,
            keyboard_focus_state: KeyboardFocusState::Nothing,
            peg_hotkeys: PegHotKeys::new(),
            pending_changes: Vec::new(),
            selection: HashSet::new(),
            graph_topology: topology,
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
        self.update_keyboard_focus_from_selection();
    }

    pub fn select_object(&mut self, object_id: ObjectId) {
        self.selection.insert(object_id);
        self.update_keyboard_focus_from_selection();
    }

    pub fn deselect_object(&mut self, object_id: ObjectId) {
        self.selection.remove(&object_id);
        self.update_keyboard_focus_from_selection();
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
        self.update_keyboard_focus_from_selection();
    }

    pub fn forget_selection(&mut self) {
        for id in &self.selection {
            self.layout_state.forget_object(*id);
            self.object_states.remove(id);
        }
        self.selection.clear();
        self.update_keyboard_focus_from_selection();
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

    fn update_keyboard_focus_from_selection(&mut self) {
        self.keyboard_focus_state = KeyboardFocusState::Nothing;
        if self.selection.len() != 1 {
            self.update_peg_hotkeys_from_keyboard_focus();
            return;
        }
        let selected_object = *self.selection.iter().next().unwrap();
        self.keyboard_focus_state = match selected_object {
            ObjectId::Sound(spid) => KeyboardFocusState::SoundProcessor(spid),
            ObjectId::Number(nsid) => KeyboardFocusState::NumberSource(nsid),
        };
        self.update_peg_hotkeys_from_keyboard_focus();
    }

    fn update_peg_hotkeys_from_keyboard_focus(&mut self) {
        let mut available_pegs: Vec<(GraphId, HotKeyAction)> = Vec::new();
        let topo = self.graph_topology.read();
        match self.keyboard_focus_state {
            KeyboardFocusState::Nothing => (),
            KeyboardFocusState::SoundProcessor(spid) => {
                let sp = topo.sound_processors().get(&spid).unwrap();
                // TODO: if the processor produces output, add all sound inputs that it can legally be connected to
                // TODO: if the processor has exactly one input, add all sound outputs that can legally connect to it
                // TODO: don't add the processor's id if it doesn't produce output
                available_pegs.push((spid.into(), HotKeyAction::Activate(spid.into())));
                for si in sp.sound_inputs() {
                    available_pegs.push((si.into(), HotKeyAction::Activate(si.into())));
                }
                for ni in sp.number_inputs() {
                    available_pegs.push((ni.into(), HotKeyAction::Activate(ni.into())));
                }
                for ns in sp.number_sources() {
                    available_pegs.push((ns.into(), HotKeyAction::Activate(ns.into())));
                }
            }
            KeyboardFocusState::NumberSource(nsid) => {
                // TODO: add all number inputs that the number source's output can legally be connected to
                // TODO: if the number source has exactly one input, add all number outputs that can legally connect to it
                let ns = topo.number_sources().get(&nsid).unwrap();
                available_pegs.push((nsid.into(), HotKeyAction::Activate(nsid.into())));
                for ni in ns.inputs() {
                    available_pegs.push((ni.into(), HotKeyAction::Activate(ni.into())));
                }
            }
            KeyboardFocusState::SoundInput(siid) => {
                for spid in topo.sound_processors().keys() {
                    if topo.is_legal_sound_connection(siid, *spid) {
                        available_pegs
                            .push((spid.into(), HotKeyAction::Connect(siid.into(), spid.into())));
                    }
                }
            }
            KeyboardFocusState::SoundOutput(spid) => {
                for siid in topo.sound_inputs().keys() {
                    if topo.is_legal_sound_connection(*siid, spid) {
                        available_pegs
                            .push((siid.into(), HotKeyAction::Connect(spid.into(), siid.into())));
                    }
                }
            }
            KeyboardFocusState::NumberInput(niid) => {
                for nsid in topo.number_sources().keys() {
                    if topo.is_legal_number_connection(niid, *nsid) {
                        available_pegs
                            .push((nsid.into(), HotKeyAction::Connect(niid.into(), nsid.into())));
                    }
                }
            }
            KeyboardFocusState::NumberOutput(nsid) => {
                for niid in topo.number_inputs().keys() {
                    if topo.is_legal_number_connection(*niid, nsid) {
                        available_pegs
                            .push((niid.into(), HotKeyAction::Connect(nsid.into(), niid.into())));
                    }
                }
            }
        }
        // TODO: try to persist hotkeys of pegs that remain available
        self.peg_hotkeys.clear();
        self.peg_hotkeys
            .replace(self.assign_hotkeys_to_pegs(&available_pegs));
    }

    fn assign_hotkeys_to_pegs(
        &self,
        pegs_actions: &[(GraphId, HotKeyAction)],
    ) -> HashMap<GraphId, (egui::Key, HotKeyAction)> {
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
                mapping.insert(*p, (*k, *a));
            } else {
                break;
            }
        }
        mapping
    }

    pub fn object_has_keyboard_focus(&self, object: ObjectId) -> bool {
        match (object, self.keyboard_focus_state) {
            (ObjectId::Sound(spid1), KeyboardFocusState::SoundProcessor(spid2)) => spid1 == spid2,
            (ObjectId::Number(nsid1), KeyboardFocusState::NumberSource(nsid2)) => nsid1 == nsid2,
            _ => false,
        }
    }

    pub fn peg_has_keyboard_focus(&self, id: GraphId) -> bool {
        match (id, self.keyboard_focus_state) {
            (GraphId::NumberInput(i1), KeyboardFocusState::NumberInput(i2)) => i1 == i2,
            (GraphId::NumberSource(i1), KeyboardFocusState::NumberOutput(i2)) => i1 == i2,
            (GraphId::SoundInput(i1), KeyboardFocusState::SoundInput(i2)) => i1 == i2,
            (GraphId::SoundProcessor(i1), KeyboardFocusState::SoundOutput(i2)) => i1 == i2,
            (_, _) => false,
        }
    }

    pub fn peg_has_hotkey(&self, id: GraphId) -> Option<egui::Key> {
        self.peg_hotkeys.mapping.get(&id).map(|x| x.0)
    }

    pub fn activate_hotkey(&mut self, key: egui::Key, graph: &mut SoundGraph) {
        let action =
            self.peg_hotkeys
                .mapping
                .iter()
                .find_map(|(_, (k, a))| if *k == key { Some(*a) } else { None });
        let action = match action {
            Some(a) => a,
            None => return,
        };
        match action {
            HotKeyAction::Activate(gid) => {
                self.keyboard_focus_state = match gid {
                    GraphId::SoundInput(siid) => KeyboardFocusState::SoundInput(siid),
                    GraphId::SoundProcessor(spid) => KeyboardFocusState::SoundOutput(spid),
                    GraphId::NumberInput(niid) => KeyboardFocusState::NumberInput(niid),
                    GraphId::NumberSource(nsid) => KeyboardFocusState::NumberOutput(nsid),
                };
            }
            HotKeyAction::Connect(gid1, gid2) => {
                match (gid1, gid2) {
                    (GraphId::NumberInput(niid), GraphId::NumberSource(nsid)) => {
                        graph.disconnect_number_input(niid).unwrap();
                        graph.connect_number_input(niid, nsid).unwrap();
                        self.keyboard_focus_state = KeyboardFocusState::NumberSource(nsid);
                    }
                    (GraphId::NumberSource(nsid), GraphId::NumberInput(niid)) => {
                        graph.disconnect_number_input(niid).unwrap();
                        graph.connect_number_input(niid, nsid).unwrap();
                        let owner = graph
                            .topology()
                            .read()
                            .number_inputs()
                            .get(&niid)
                            .unwrap()
                            .owner();
                        self.keyboard_focus_state = match owner {
                            NumberInputOwner::SoundProcessor(i) => {
                                KeyboardFocusState::SoundProcessor(i)
                            }
                            NumberInputOwner::NumberSource(i) => {
                                KeyboardFocusState::NumberSource(i)
                            }
                        };
                    }
                    (GraphId::SoundInput(siid), GraphId::SoundProcessor(spid)) => {
                        graph.disconnect_sound_input(siid).unwrap();
                        graph.connect_sound_input(siid, spid).unwrap();
                        self.keyboard_focus_state = KeyboardFocusState::SoundProcessor(spid);
                    }
                    (GraphId::SoundProcessor(spid), GraphId::SoundInput(siid)) => {
                        graph.disconnect_sound_input(siid).unwrap();
                        graph.connect_sound_input(siid, spid).unwrap();
                        let owner = graph
                            .topology()
                            .read()
                            .sound_inputs()
                            .get(&siid)
                            .unwrap()
                            .owner();
                        self.keyboard_focus_state = KeyboardFocusState::SoundProcessor(owner);
                    }
                    (_, _) => panic!(),
                };
            }
        }
        self.update_peg_hotkeys_from_keyboard_focus();
    }

    pub fn cancel_hotkey(&mut self, graph: &SoundGraph) {
        let topo = graph.topology();
        let topo = topo.read();
        self.keyboard_focus_state = match self.keyboard_focus_state {
            KeyboardFocusState::SoundInput(siid) => {
                let o = topo.sound_inputs().get(&siid).unwrap().owner();
                KeyboardFocusState::SoundProcessor(o)
            }
            KeyboardFocusState::SoundOutput(spid) => KeyboardFocusState::SoundProcessor(spid),
            KeyboardFocusState::NumberInput(niid) => {
                let o = topo.number_inputs().get(&niid).unwrap().owner();
                match o {
                    NumberInputOwner::SoundProcessor(i) => KeyboardFocusState::SoundProcessor(i),
                    NumberInputOwner::NumberSource(i) => KeyboardFocusState::NumberSource(i),
                }
            }
            KeyboardFocusState::NumberOutput(nsid) => {
                let o = topo.number_sources().get(&nsid).unwrap().owner();
                match o {
                    NumberSourceOwner::Nothing => panic!(),
                    NumberSourceOwner::SoundProcessor(i) => KeyboardFocusState::SoundProcessor(i),
                    NumberSourceOwner::SoundInput(i) => KeyboardFocusState::SoundProcessor(
                        topo.sound_inputs().get(&i).unwrap().owner(),
                    ),
                }
            }
            KeyboardFocusState::Nothing => KeyboardFocusState::Nothing,
            KeyboardFocusState::SoundProcessor(_) => KeyboardFocusState::Nothing,
            KeyboardFocusState::NumberSource(_) => KeyboardFocusState::Nothing,
        };
        self.update_peg_hotkeys_from_keyboard_focus();
    }

    pub(super) fn apply_pending_changes(&mut self, graph: &mut SoundGraph) {
        for f in self.pending_changes.drain(..) {
            f(graph);
        }
        debug_assert!(self.pending_changes.len() == 0);
    }
}
