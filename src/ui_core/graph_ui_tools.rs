use std::collections::HashMap;

use eframe::egui;

use crate::core::{
    graphobject::GraphId, numberinput::NumberInputId, numbersource::NumberSourceId,
    soundgraph::SoundGraph, soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

pub struct PegState {
    pub rect: egui::Rect,
    pub layer: egui::LayerId,
}

impl PegState {
    pub fn center(&self) -> egui::Pos2 {
        self.rect.center()
    }
}

pub struct PegStateMap<T: UniqueId> {
    states: HashMap<T, PegState>,
}

impl<T: UniqueId> PegStateMap<T> {
    pub fn new() -> PegStateMap<T> {
        PegStateMap {
            states: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.states.clear();
    }

    fn states(&self) -> &HashMap<T, PegState> {
        &self.states
    }

    pub fn add(&mut self, id: T, rect: egui::Rect, layer: egui::LayerId) {
        let state = PegState { rect, layer };
        self.states.insert(id, state);
    }
}

pub struct GraphUITools {
    sound_inputs: PegStateMap<SoundInputId>,
    sound_outputs: PegStateMap<SoundProcessorId>,
    number_inputs: PegStateMap<NumberInputId>,
    number_outputs: PegStateMap<NumberSourceId>,
    peg_being_dragged: Option<GraphId>,
    dropped_peg: Option<(GraphId, egui::Pos2)>,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph) -> ()>>,
}

impl GraphUITools {
    pub(super) fn new() -> GraphUITools {
        GraphUITools {
            sound_inputs: PegStateMap::new(),
            sound_outputs: PegStateMap::new(),
            number_inputs: PegStateMap::new(),
            number_outputs: PegStateMap::new(),
            peg_being_dragged: None,
            dropped_peg: None,
            pending_changes: Vec::new(),
        }
    }

    pub(super) fn reset(&mut self) {
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

    pub fn make_change<F: FnOnce(&mut SoundGraph) -> () + 'static>(&mut self, f: F) {
        self.pending_changes.push(Box::new(f));
    }

    pub(super) fn sound_inputs(&self) -> &HashMap<SoundInputId, PegState> {
        &self.sound_inputs.states
    }

    pub(super) fn sound_outputs(&self) -> &HashMap<SoundProcessorId, PegState> {
        &self.sound_outputs.states
    }

    pub(super) fn number_inputs(&self) -> &HashMap<NumberInputId, PegState> {
        &self.number_inputs.states
    }

    pub(super) fn number_outputs(&self) -> &HashMap<NumberSourceId, PegState> {
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
            peg_states: &PegStateMap<T>,
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

    pub(super) fn apply_pending_changes(&mut self, graph: &mut SoundGraph) {
        for f in self.pending_changes.drain(..) {
            f(graph);
        }
        debug_assert!(self.pending_changes.len() == 0);
    }
}
