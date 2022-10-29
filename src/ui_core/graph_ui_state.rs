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
    graphserialization::{ForwardGraphIdMap, ReverseGraphIdMap},
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSourceId, NumberSourceOwner},
    serialization::{Deserializer, Serializable, Serializer},
    soundgraph::SoundGraph,
    soundgraphdescription::SoundGraphDescription,
    soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

use super::ui_factory::UiFactory;

fn serialize_object_id(id: ObjectId, serializer: &mut Serializer, idmap: &ForwardGraphIdMap) {
    match id {
        ObjectId::Sound(i) => {
            serializer.u8(1);
            serializer.u16(idmap.sound_processors().map_id(i).unwrap());
        }
        ObjectId::Number(i) => {
            serializer.u8(2);
            serializer.u16(idmap.number_sources().map_id(i).unwrap());
        }
    }
}

fn deserialize_object_id(
    deserializer: &mut Deserializer,
    idmap: &ReverseGraphIdMap,
) -> Result<ObjectId, ()> {
    Ok(match deserializer.u8()? {
        1 => ObjectId::Sound(idmap.sound_processors().map_id(deserializer.u16()?)),
        2 => ObjectId::Number(idmap.number_sources().map_id(deserializer.u16()?).into()),
        _ => return Err(()),
    })
}

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

    fn retain(&mut self, ids: &HashSet<GraphId>) {
        self.objects
            .states_mut()
            .retain(|i, _| ids.contains(&(*i).into()));
        self.sound_inputs
            .states_mut()
            .retain(|i, _| ids.contains(&(*i).into()));
        self.sound_outputs
            .states_mut()
            .retain(|i, _| ids.contains(&(*i).into()));
        self.number_inputs
            .states_mut()
            .retain(|i, _| ids.contains(&(*i).into()));
        self.number_outputs
            .states_mut()
            .retain(|i, _| ids.contains(&(*i).into()));
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
        let rad = ui.input().aim_radius();
        let top_layer = match ui.memory().layer_id_at(position, rad) {
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

    fn serialize_positions(
        &self,
        serializer: &mut Serializer,
        subset: Option<&HashSet<ObjectId>>,
        idmap: &ForwardGraphIdMap,
    ) {
        let is_selected = |id: ObjectId| match subset {
            Some(s) => s.get(&id).is_some(),
            None => true,
        };
        let mut s1 = serializer.subarchive();
        for (id, layout) in &self.objects.states {
            if !is_selected(*id) {
                continue;
            }
            serialize_object_id(*id, &mut s1, idmap);
            s1.f32(layout.rect.left());
            s1.f32(layout.rect.top());
        }
    }

    fn deserialize_positions(
        &mut self,
        deserializer: &mut Deserializer,
        idmap: &ReverseGraphIdMap,
    ) -> Result<(), ()> {
        let mut d1 = deserializer.subarchive()?;
        while !d1.is_empty() {
            let id: ObjectId = deserialize_object_id(&mut d1, idmap)?;
            let left = d1.f32()?;
            let top = d1.f32()?;
            let layout = self.objects.states.entry(id).or_insert(LayoutState {
                rect: egui::Rect::NAN,
                layer: egui::LayerId::debug(),
            });
            layout.rect.set_left(left);
            layout.rect.set_top(top);
        }
        Ok(())
    }
}

pub trait ObjectUiState: 'static {
    fn as_any(&self) -> &dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, serializer: &mut Serializer);
}

impl<T: 'static + Serializable> ObjectUiState for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, serializer: &mut Serializer) {
        serializer.object(self);
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
    fn new() -> PegHotKeys {
        PegHotKeys {
            mapping: HashMap::new(),
        }
    }

    fn replace(&mut self, mapping: HashMap<GraphId, (egui::Key, HotKeyAction)>) {
        self.mapping = mapping;
    }

    fn retain(&mut self, ids: &HashSet<GraphId>) {
        self.mapping.retain(|i, _| ids.contains(i));
    }

    fn clear(&mut self) {
        self.mapping.clear();
    }
}

#[derive(Copy, Clone)]
enum KeyboardFocusState {
    SoundProcessor(SoundProcessorId),
    NumberSource(NumberSourceId),
    SoundInput(SoundInputId),
    SoundOutput(SoundProcessorId),
    NumberInput(NumberInputId),
    NumberOutput(NumberSourceId),
}

impl KeyboardFocusState {
    fn as_graph_id(&self) -> GraphId {
        match self {
            KeyboardFocusState::SoundProcessor(i) => (*i).into(),
            KeyboardFocusState::NumberSource(i) => (*i).into(),
            KeyboardFocusState::SoundInput(i) => (*i).into(),
            KeyboardFocusState::SoundOutput(i) => (*i).into(),
            KeyboardFocusState::NumberInput(i) => (*i).into(),
            KeyboardFocusState::NumberOutput(i) => (*i).into(),
        }
    }
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
    object_states: HashMap<ObjectId, Rc<RefCell<dyn ObjectUiState>>>,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph) -> ()>>,
    mode: UiMode,
    graph_topology: Arc<RwLock<SoundGraphTopology>>,
    ui_factory: Arc<RwLock<UiFactory>>,
}

impl GraphUIState {
    pub(super) fn new(
        topology: Arc<RwLock<SoundGraphTopology>>,
        object_factory: Arc<RwLock<UiFactory>>,
    ) -> GraphUIState {
        GraphUIState {
            layout_state: GraphLayout::new(),
            object_states: HashMap::new(),
            pending_changes: Vec::new(),
            mode: UiMode::Passive,
            graph_topology: topology,
            ui_factory: object_factory,
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

    pub fn set_object_state(&mut self, id: ObjectId, state: Rc<RefCell<dyn ObjectUiState>>) {
        self.object_states.insert(id, state);
    }

    pub fn get_object_state(&mut self, id: ObjectId) -> Rc<RefCell<dyn ObjectUiState>> {
        Rc::clone(self.object_states.get(&id).unwrap())
    }

    pub fn make_change<F: FnOnce(&mut SoundGraph) -> () + 'static>(&mut self, f: F) {
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

    pub(super) fn peg_being_dropped(&self) -> Option<(GraphId, egui::Pos2)> {
        match self.mode {
            UiMode::DroppingPeg(p) => Some(p),
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
        for (object_id, object_state) in self.layout_state.objects().states() {
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

    pub fn cleanup(&mut self) {
        let remaining_ids;
        {
            let mut ids: HashSet<GraphId> = HashSet::new();
            let topo = self.graph_topology.read();
            ids.extend(
                topo.sound_processors()
                    .keys()
                    .map(|i| -> GraphId { (*i).into() }),
            );
            ids.extend(
                topo.sound_inputs()
                    .keys()
                    .map(|i| -> GraphId { (*i).into() }),
            );
            ids.extend(
                topo.number_sources()
                    .keys()
                    .map(|i| -> GraphId { (*i).into() }),
            );
            ids.extend(
                topo.number_inputs()
                    .keys()
                    .map(|i| -> GraphId { (*i).into() }),
            );
            remaining_ids = ids;
        }

        self.layout_state.retain(&remaining_ids);
        self.object_states
            .retain(|i, _| remaining_ids.contains(&(*i).into()));

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
                    let state = objects.states_mut().get_mut(s).unwrap();
                    state.rect = state.rect.translate(delta);
                }
            }
            _ => (),
        }
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

    fn update_peg_hotkeys_from_keyboard_focus(&mut self) {
        let (keyboard_focus_state, peg_hotkeys) = match &mut self.mode {
            UiMode::UsingKeyboardNav(x) => x,
            _ => return,
        };

        let mut available_pegs: Vec<(GraphId, HotKeyAction)> = Vec::new();
        let topo = self.graph_topology.read();
        match *keyboard_focus_state {
            KeyboardFocusState::SoundProcessor(spid) => {
                let sp = topo.sound_processors().get(&spid).unwrap();
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
                let ns = topo.number_sources().get(&nsid).unwrap();
                available_pegs.push((nsid.into(), HotKeyAction::Activate(nsid.into())));
                for ni in ns.inputs().iter().cloned() {
                    available_pegs.push((ni.into(), HotKeyAction::Activate(ni.into())));
                }
            }
            KeyboardFocusState::SoundInput(siid) => {
                for spid in topo.sound_processors().keys().cloned() {
                    if topo.is_legal_sound_connection(siid, spid) {
                        available_pegs
                            .push((spid.into(), HotKeyAction::Connect(siid.into(), spid.into())));
                    }
                }
            }
            KeyboardFocusState::SoundOutput(spid) => {
                for siid in topo.sound_inputs().keys().cloned() {
                    if topo.is_legal_sound_connection(siid, spid) {
                        available_pegs
                            .push((siid.into(), HotKeyAction::Connect(spid.into(), siid.into())));
                    }
                }
            }
            KeyboardFocusState::NumberInput(niid) => {
                for nsid in topo.number_sources().keys().cloned() {
                    if topo.is_legal_number_connection(niid, nsid) {
                        available_pegs
                            .push((nsid.into(), HotKeyAction::Connect(niid.into(), nsid.into())));
                    }
                }
            }
            KeyboardFocusState::NumberOutput(nsid) => {
                for niid in topo.number_inputs().keys().cloned() {
                    if topo.is_legal_number_connection(niid, nsid) {
                        available_pegs
                            .push((niid.into(), HotKeyAction::Connect(nsid.into(), niid.into())));
                    }
                }
            }
        }
        // TODO: try to persist hotkeys of pegs that remain available
        peg_hotkeys.clear();
        peg_hotkeys.replace(Self::assign_hotkeys_to_pegs(&available_pegs));
    }

    fn assign_hotkeys_to_pegs(
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
        let keyboard_focus = match &self.mode {
            UiMode::UsingKeyboardNav(p) => &p.0,
            _ => return false,
        };
        match (object, keyboard_focus) {
            (ObjectId::Sound(spid1), KeyboardFocusState::SoundProcessor(spid2)) => spid1 == *spid2,
            (ObjectId::Number(nsid1), KeyboardFocusState::NumberSource(nsid2)) => nsid1 == *nsid2,
            _ => false,
        }
    }

    pub fn peg_has_keyboard_focus(&self, id: GraphId) -> bool {
        let keyboard_focus = match &self.mode {
            UiMode::UsingKeyboardNav(p) => &p.0,
            _ => return false,
        };
        match (id, keyboard_focus) {
            (GraphId::NumberInput(i1), KeyboardFocusState::NumberInput(i2)) => i1 == *i2,
            (GraphId::NumberSource(i1), KeyboardFocusState::NumberOutput(i2)) => i1 == *i2,
            (GraphId::SoundInput(i1), KeyboardFocusState::SoundInput(i2)) => i1 == *i2,
            (GraphId::SoundProcessor(i1), KeyboardFocusState::SoundOutput(i2)) => i1 == *i2,
            (_, _) => false,
        }
    }

    pub fn peg_has_hotkey(&self, id: GraphId) -> Option<egui::Key> {
        match &self.mode {
            UiMode::UsingKeyboardNav(x) => x.1.mapping.get(&id).map(|x| x.0),
            _ => None,
        }
    }

    pub(super) fn activate_hotkey(&mut self, key: egui::Key, desc: &SoundGraphDescription) -> bool {
        let (keyboard_focus, peg_hotkeys) = match &mut self.mode {
            UiMode::UsingKeyboardNav(p) => p,
            _ => return false,
        };

        let action = peg_hotkeys
            .mapping
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
                        self.pending_changes.push(Box::new(move |g| {
                            g.disconnect_number_input(niid).unwrap();
                            g.connect_number_input(niid, nsid).unwrap();
                        }));
                        *keyboard_focus = KeyboardFocusState::NumberSource(nsid);
                    }
                    (GraphId::NumberSource(nsid), GraphId::NumberInput(niid)) => {
                        self.pending_changes.push(Box::new(move |g| {
                            g.disconnect_number_input(niid).unwrap();
                            g.connect_number_input(niid, nsid).unwrap();
                        }));
                        let owner = desc.number_inputs().get(&niid).unwrap().owner();
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
                        self.pending_changes.push(Box::new(move |g| {
                            g.disconnect_sound_input(siid).unwrap();
                            g.connect_sound_input(siid, spid).unwrap();
                        }));
                        *keyboard_focus = KeyboardFocusState::SoundProcessor(spid);
                    }
                    (GraphId::SoundProcessor(spid), GraphId::SoundInput(siid)) => {
                        self.pending_changes.push(Box::new(move |g| {
                            g.disconnect_sound_input(siid).unwrap();
                            g.connect_sound_input(siid, spid).unwrap();
                        }));
                        let owner = desc.sound_inputs().get(&siid).unwrap().owner();
                        *keyboard_focus = KeyboardFocusState::SoundProcessor(owner);
                    }
                    (_, _) => panic!(),
                };
            }
        }
        self.update_peg_hotkeys_from_keyboard_focus();
        true
    }

    pub(super) fn cancel_hotkey(&mut self, desc: &SoundGraphDescription) -> bool {
        let keyboard_focus = match &mut self.mode {
            UiMode::UsingKeyboardNav(p) => &mut p.0,
            _ => return false,
        };

        match keyboard_focus {
            KeyboardFocusState::SoundInput(siid) => {
                let o = desc.sound_inputs().get(&siid).unwrap().owner();
                *keyboard_focus = KeyboardFocusState::SoundProcessor(o);
            }
            KeyboardFocusState::SoundOutput(spid) => {
                *keyboard_focus = KeyboardFocusState::SoundProcessor(*spid);
            }
            KeyboardFocusState::NumberInput(niid) => {
                let o = desc.number_inputs().get(&niid).unwrap().owner();
                *keyboard_focus = match o {
                    NumberInputOwner::SoundProcessor(i) => KeyboardFocusState::SoundProcessor(i),
                    NumberInputOwner::NumberSource(i) => KeyboardFocusState::NumberSource(i),
                };
            }
            KeyboardFocusState::NumberOutput(nsid) => {
                let o = desc.number_sources().get(&nsid).unwrap().owner();
                *keyboard_focus = match o {
                    NumberSourceOwner::Nothing => panic!(),
                    NumberSourceOwner::SoundProcessor(i) => KeyboardFocusState::SoundProcessor(i),
                    NumberSourceOwner::SoundInput(i) => KeyboardFocusState::SoundProcessor(
                        desc.sound_inputs().get(&i).unwrap().owner(),
                    ),
                };
            }
            KeyboardFocusState::SoundProcessor(_) => {
                self.mode = UiMode::Passive;
            }
            KeyboardFocusState::NumberSource(_) => {
                self.mode = UiMode::Passive;
            }
        };
        self.update_peg_hotkeys_from_keyboard_focus();
        true
    }

    pub(super) fn apply_pending_changes(&mut self, graph: &mut SoundGraph) {
        for f in self.pending_changes.drain(..) {
            f(graph);
        }
        debug_assert!(self.pending_changes.is_empty());
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self) -> bool {
        let topo = self.graph_topology.read();
        let mut good = true;
        for i in topo.sound_processors().keys() {
            if !self.object_states.contains_key(&i.into()) {
                println!("Sound processor {} does not have a ui state", i.0);
                good = false;
            }
        }
        for (i, ns) in topo.number_sources() {
            if ns.owner() == NumberSourceOwner::Nothing {
                if !self.object_states.contains_key(&i.into()) {
                    println!("Pure number source {} does not have a ui state", i.0);
                    good = false;
                }
            }
        }
        for i in self.object_states.keys() {
            match i {
                ObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(i) {
                        println!("A ui state exists for non-existent sound processor {}", i.0);
                        good = false;
                    }
                }
                ObjectId::Number(i) => {
                    if !topo.number_sources().contains_key(i) {
                        println!("A ui state exists for non-existent number source {}", i.0);
                        good = false;
                    }
                }
            }
        }
        for i in self.layout_state.objects().states().keys() {
            match i {
                ObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(i) {
                        println!(
                            "An object position exists for a non-existent sound processor {}",
                            i.0
                        );
                        good = false;
                    }
                }
                ObjectId::Number(i) => {
                    if !topo.number_sources().contains_key(i) {
                        println!(
                            "An object position exists for a non-existent number source {}",
                            i.0
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
                    i.0
                );
                good = false;
            }
        }
        for i in self.layout_state.sound_inputs().keys() {
            if !topo.sound_inputs().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent sound input {}",
                    i.0
                );
                good = false;
            }
        }
        for i in self.layout_state.number_outputs().keys() {
            if !topo.number_sources().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent number output {}",
                    i.0
                );
                good = false;
            }
        }
        for i in self.layout_state.number_inputs().keys() {
            if !topo.number_inputs().contains_key(i) {
                println!(
                    "A screen position exists for a non-existent number output {}",
                    i.0
                );
                good = false;
            }
        }
        good
    }

    pub(super) fn serialize_ui_states(
        &self,
        serializer: &mut Serializer,
        subset: Option<&HashSet<ObjectId>>,
        idmap: &ForwardGraphIdMap,
    ) {
        let is_selected = |id: ObjectId| match subset {
            Some(s) => s.get(&id).is_some(),
            None => true,
        };
        self.layout_state
            .serialize_positions(serializer, subset, idmap);
        let mut s1 = serializer.subarchive();
        for (id, state) in &self.object_states {
            if !is_selected(*id) {
                continue;
            }
            serialize_object_id(*id, &mut s1, idmap);
            let mut s2 = s1.subarchive();
            state.borrow().serialize(&mut s2);
        }
    }

    pub(super) fn deserialize_ui_states(
        &mut self,
        deserializer: &mut Deserializer,
        idmap: &ReverseGraphIdMap,
        topology: &SoundGraphTopology,
        ui_factory: &UiFactory,
    ) -> Result<(), ()> {
        self.layout_state
            .deserialize_positions(deserializer, idmap)?;
        let mut d1 = deserializer.subarchive()?;
        while !d1.is_empty() {
            let id = deserialize_object_id(&mut d1, idmap)?;
            let obj = match id {
                ObjectId::Sound(i) => match topology.sound_processors().get(&i) {
                    Some(sp) => sp.instance_arc().as_graph_object(),
                    None => return Err(()),
                },
                ObjectId::Number(i) => match topology.number_sources().get(&i) {
                    Some(ns) => {
                        if let Some(o) = ns.instance_arc().as_graph_object(i) {
                            o
                        } else {
                            return Err(());
                        }
                    }
                    None => return Err(()),
                },
            };
            let d2 = d1.subarchive()?;
            let state = ui_factory.create_state_from_archive(&*obj, d2)?;
            self.set_object_state(id, state);
        }
        Ok(())
    }

    pub fn select_all(&mut self) {
        let mut ids: HashSet<ObjectId> = HashSet::new();
        {
            let topo = self.graph_topology.read();
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

    pub fn select_none(&mut self) {
        if let UiMode::Selecting(_) = self.mode {
            self.mode = UiMode::Passive;
        }
    }

    pub fn make_states_for_new_objects(&mut self) {
        let topo = self.graph_topology.read();
        for (i, spd) in topo.sound_processors() {
            self.object_states.entry(i.into()).or_insert_with(|| {
                let o = spd.instance_arc().as_graph_object();
                self.ui_factory.read().create_default_state(&*o)
            });
        }
        for (i, nsd) in topo.number_sources() {
            if nsd.owner() != NumberSourceOwner::Nothing {
                continue;
            }
            self.object_states.entry(i.into()).or_insert_with(|| {
                let o = nsd.instance_arc().as_graph_object(*i).unwrap();
                self.ui_factory.read().create_default_state(&*o)
            });
        }
    }
}
