use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::sound::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgraph::SoundGraph,
    soundgraphid::{SoundGraphId, SoundObjectId},
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::find_error,
    soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
    soundprocessor::SoundProcessorId,
};

use super::{
    hotkeys::KeyboardFocusState, numbergraphuistate::NumberGraphUiState,
    object_positions::ObjectPositions, soundnumberinputui::SoundNumberInputPresentation,
    soundobjectuistate::SoundObjectUiStates, temporallayout::TemporalLayout,
};

pub struct NestedProcessorClosure {
    pub sound_processors: HashSet<SoundProcessorId>,
    pub sound_inputs: HashSet<SoundInputId>,
}

pub struct CandidateSoundInput {
    pub score: f32,
    pub is_selected: bool,
}

pub struct DraggingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    original_rect: egui::Rect,
    pub drag_closure: NestedProcessorClosure,
    pub candidate_inputs: HashMap<SoundInputId, CandidateSoundInput>,
    pub from_input: Option<SoundInputId>,
}

pub struct DroppingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    pub target_input: Option<SoundInputId>,
    pub from_input: Option<SoundInputId>,
}

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

enum UiMode {
    Passive,
    UsingKeyboardNav(KeyboardFocusState),
    Selecting(HashSet<SoundObjectId>),
    DraggingProcessor(DraggingProcessorData),
    DroppingProcessor(DroppingProcessorData),
}

// Used to defer moving processors from the process of laying them out
struct PendingProcessorDrag {
    processor_id: SoundProcessorId,
    delta: egui::Vec2,
    cursor_pos: egui::Pos2,
    from_input: Option<SoundInputId>,
    from_rect: egui::Rect,
}

pub struct SoundGraphUiState {
    object_positions: ObjectPositions,
    temporal_layout: TemporalLayout,
    pending_changes: Vec<Box<dyn FnOnce(&mut SoundGraph, &mut SoundGraphUiState) -> ()>>,
    mode: UiMode,
    pending_drag: Option<PendingProcessorDrag>,
    number_graph_uis:
        HashMap<SoundNumberInputId, (NumberGraphUiState, SoundNumberInputPresentation)>,
}

impl SoundGraphUiState {
    pub(super) fn new() -> SoundGraphUiState {
        SoundGraphUiState {
            object_positions: ObjectPositions::new(),
            temporal_layout: TemporalLayout::new(),
            pending_changes: Vec::new(),
            mode: UiMode::Passive,
            pending_drag: None,
            number_graph_uis: HashMap::new(),
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

    pub fn make_change<F: FnOnce(&mut SoundGraph, &mut SoundGraphUiState) -> () + 'static>(
        &mut self,
        f: F,
    ) {
        self.pending_changes.push(Box::new(f));
    }

    pub(super) fn stop_selecting(&mut self) {
        match self.mode {
            UiMode::Selecting(_) => self.mode = UiMode::Passive,
            _ => (),
        }
    }

    pub(super) fn set_selection(&mut self, object_ids: HashSet<SoundObjectId>) {
        self.mode = UiMode::Selecting(object_ids);
    }

    pub(super) fn select_object(&mut self, object_id: SoundObjectId) {
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

    pub(super) fn select_with_rect(&mut self, rect: egui::Rect, change: SelectionChange) {
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
            if !self.temporal_layout.is_top_level(*object_id) {
                continue;
            }
            if rect.intersects(object_state.rect()) {
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

    fn find_nested_processor_closure(
        processor_id: SoundProcessorId,
        topo: &SoundGraphTopology,
        temporal_layout: &TemporalLayout,
    ) -> NestedProcessorClosure {
        fn visitor(
            processor_id: SoundProcessorId,
            topo: &SoundGraphTopology,
            temporal_layout: &TemporalLayout,
            closure: &mut NestedProcessorClosure,
        ) {
            closure.sound_processors.insert(processor_id);
            let inputs = topo.sound_processor(processor_id).unwrap().sound_inputs();
            for siid in inputs {
                closure.sound_inputs.insert(*siid);
                let Some(target_spid) = topo.sound_input(*siid).unwrap().target() else {
                    continue;
                };

                if temporal_layout.is_top_level(target_spid.into()) {
                    continue;
                }

                visitor(target_spid, topo, temporal_layout, closure);
            }
        }

        let mut closure = NestedProcessorClosure {
            sound_processors: HashSet::new(),
            sound_inputs: HashSet::new(),
        };

        visitor(processor_id, topo, temporal_layout, &mut closure);

        closure
    }

    fn find_candidate_sound_inputs(
        processor_id: SoundProcessorId,
        original_topo: &SoundGraphTopology,
        excluded_closure: &NestedProcessorClosure,
    ) -> HashMap<SoundInputId, CandidateSoundInput> {
        let mut topo_disconnected = original_topo.clone();
        for si_data in original_topo.sound_inputs().values() {
            if si_data.target() != Some(processor_id) {
                continue;
            }
            for (niid, nsid) in original_topo.number_connection_crossings(si_data.id()) {
                topo_disconnected
                    .make_sound_number_edit(SoundNumberEdit::DisconnectNumberInput(niid, nsid));
            }
            topo_disconnected.make_sound_edit(SoundEdit::DisconnectSoundInput(si_data.id()));
        }

        let topo_disconnected = topo_disconnected;

        debug_assert_eq!(find_error(&topo_disconnected), None);

        let mut candidates = HashMap::new();

        for si_data in topo_disconnected.sound_inputs().values() {
            if excluded_closure.sound_inputs.contains(&si_data.id()) {
                continue;
            }

            if si_data.target().is_some() {
                continue;
            }

            // try connecting the sound input in a clone of the topology,
            // mark it as a candidate if there are no errors
            let mut topo_reconnected = topo_disconnected.clone();
            topo_reconnected
                .make_sound_edit(SoundEdit::ConnectSoundInput(si_data.id(), processor_id));
            if find_error(&topo_reconnected).is_none() {
                candidates.insert(
                    si_data.id(),
                    CandidateSoundInput {
                        score: f32::INFINITY,
                        is_selected: false,
                    },
                );
            }
        }

        candidates
    }

    pub(super) fn update_candidate_input_scores(
        candidate_inputs: &mut HashMap<SoundInputId, CandidateSoundInput>,
        drag_processor_rect: egui::Rect,
        object_positions: &ObjectPositions,
        cursor_pos: egui::Pos2,
    ) {
        let mut lowest_score = f32::INFINITY;
        let mut lowest_scoring_input = None;

        for (siid, input_data) in candidate_inputs.iter_mut() {
            let input_layout = object_positions.get_sound_input_location(*siid).unwrap();

            let intersection = drag_processor_rect.intersect(input_layout.rect());
            if intersection.is_negative() {
                input_data.score = f32::INFINITY;
                input_data.is_selected = false;
                continue;
            }

            let intersection_score = -intersection.area().sqrt();
            let cursor_distance_score = input_layout.rect().signed_distance_to_pos(cursor_pos);
            let score = intersection_score + cursor_distance_score;
            if score < lowest_score {
                lowest_score = score;
                lowest_scoring_input = Some(*siid);
            }

            input_data.score = score;
            input_data.is_selected = false;
        }

        if let Some(siid) = lowest_scoring_input {
            // feels about right
            if lowest_score < -30.0 {
                candidate_inputs.get_mut(&siid).unwrap().is_selected = true;
            }
        }
    }

    pub(super) fn drag_processor(
        &mut self,
        processor_id: SoundProcessorId,
        delta: egui::Vec2,
        cursor_pos: egui::Pos2,
        from_input: Option<SoundInputId>,
        from_rect: egui::Rect,
    ) {
        self.pending_drag = Some(PendingProcessorDrag {
            processor_id,
            delta,
            cursor_pos,
            from_input,
            from_rect,
        });
    }

    pub(super) fn apply_processor_drag(&mut self, ui: &egui::Ui, topo: &SoundGraphTopology) {
        let Some(pending_drag) = self.pending_drag.take() else {
            return;
        };

        let PendingProcessorDrag {
            processor_id,
            delta,
            cursor_pos,
            from_input,
            from_rect,
        } = pending_drag;

        if let UiMode::Selecting(_) = &self.mode {
            self.move_selection(delta, topo);
            return;
        }

        let get_default_data = || {
            // let rect = self
            //     .object_positions
            //     .get_object_location(processor_id.into())
            //     .unwrap()
            //     .rect();
            let rect = from_rect;
            let drag_closure =
                Self::find_nested_processor_closure(processor_id, topo, &self.temporal_layout);
            let candidate_inputs =
                Self::find_candidate_sound_inputs(processor_id, topo, &drag_closure);
            DraggingProcessorData {
                processor_id,
                rect,
                original_rect: rect,
                drag_closure,
                candidate_inputs,
                from_input,
            }
        };

        // Assumption: sound graph topology isn't changing while processor is being dragged,
        // so candidate inputs don't need recomputing

        let mode = std::mem::replace(&mut self.mode, UiMode::Passive);
        let mut data = match mode {
            UiMode::DraggingProcessor(data) => {
                if data.processor_id == processor_id {
                    data
                } else {
                    get_default_data()
                }
            }
            _ => get_default_data(),
        };

        data.rect = data.rect.translate(delta);

        // If the processor is top level and shift isn't held, move it
        let shift_is_down = ui.input(|i| i.modifiers.shift);

        if self.temporal_layout.is_top_level(processor_id.into()) && from_input.is_none() {
            if shift_is_down {
                self.object_positions
                    .track_object_location(processor_id.into(), data.original_rect);
            } else {
                self.object_positions.move_sound_processor_closure(
                    processor_id.into(),
                    topo,
                    &self.temporal_layout,
                    delta,
                );
            }
        }

        Self::update_candidate_input_scores(
            &mut data.candidate_inputs,
            data.rect,
            &self.object_positions,
            cursor_pos,
        );

        self.mode = UiMode::DraggingProcessor(data);
    }

    pub(super) fn drop_dragging_processor(&mut self) {
        if let UiMode::DraggingProcessor(data) = &self.mode {
            self.mode = UiMode::DroppingProcessor(DroppingProcessorData {
                processor_id: data.processor_id,
                rect: data.rect,
                target_input: data
                    .candidate_inputs
                    .iter()
                    .filter_map(|(siid, d)| if d.is_selected { Some(*siid) } else { None })
                    .next(),
                from_input: data.from_input,
            });
        }
    }

    pub(super) fn take_dropped_nested_processor(&mut self) -> Option<DroppingProcessorData> {
        let mode = std::mem::replace(&mut self.mode, UiMode::Passive);
        match mode {
            UiMode::DroppingProcessor(data) => {
                self.mode = UiMode::Passive;
                Some(data)
            }
            _ => {
                self.mode = mode;
                None
            }
        }
    }

    pub(super) fn dragging_processor_data(&self) -> Option<&DraggingProcessorData> {
        match &self.mode {
            UiMode::DraggingProcessor(data) => Some(data),
            _ => None,
        }
    }

    pub(super) fn cleanup(
        &mut self,
        // TODO: remove this hashset completely here and elsewhere, refer to topology only
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
            UiMode::DraggingProcessor(data) => {
                if !remaining_ids.contains(&data.processor_id.into()) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::DroppingProcessor(data) => {
                if !remaining_ids.contains(&data.processor_id.into()) {
                    self.mode = UiMode::Passive;
                }
            }
        }

        // TODO: do this conservatively, e.g. when the topology changes
        self.temporal_layout.regenerate(topo);

        self.number_graph_uis
            .retain(|id, _| topo.number_inputs().contains_key(id));

        for (niid, (number_ui_state, presentation)) in &mut self.number_graph_uis {
            let number_topo = topo.number_input(*niid).unwrap().number_graph().topology();
            number_ui_state.cleanup(number_topo);
            presentation.cleanup(number_topo);
        }
    }

    pub(super) fn selection(&self) -> HashSet<SoundObjectId> {
        // TODO: this is silly, don't clone the selection.
        match &self.mode {
            UiMode::Selecting(s) => s.clone(),
            _ => HashSet::new(),
        }
    }

    pub(super) fn is_object_selected(&self, object_id: SoundObjectId) -> bool {
        match &self.mode {
            UiMode::Selecting(s) => s.contains(&object_id),
            _ => false,
        }
    }

    pub(super) fn is_object_only_selected(&self, object_id: SoundObjectId) -> bool {
        match &self.mode {
            UiMode::Selecting(s) => s.len() == 1 && s.contains(&object_id),
            _ => false,
        }
    }

    pub(super) fn move_selection(&mut self, delta: egui::Vec2, topo: &SoundGraphTopology) {
        match &self.mode {
            UiMode::Selecting(selection) => {
                for s in selection {
                    if self.temporal_layout.is_top_level((*s).into()) {
                        match s {
                            SoundObjectId::Sound(spid) => {
                                self.object_positions.move_sound_processor_closure(
                                    *spid,
                                    topo,
                                    &self.temporal_layout,
                                    delta,
                                )
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    pub(super) fn object_has_keyboard_focus(&self, object_id: SoundObjectId) -> bool {
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
                SoundObjectId::Sound(i) => {
                    if !topo.sound_processors().contains_key(&i) {
                        good = false;
                    }
                }
            }
        }

        good
    }

    pub(super) fn select_all(&mut self, topo: &SoundGraphTopology) {
        let mut ids: HashSet<SoundObjectId> = HashSet::new();
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

    pub(super) fn create_state_for(
        &mut self,
        object_id: SoundObjectId,
        topo: &SoundGraphTopology,
        object_ui_states: &SoundObjectUiStates,
    ) {
        self.object_positions.create_state_for(object_id);

        match object_id {
            SoundObjectId::Sound(spid) => {
                let number_input_ids = topo.sound_processor(spid).unwrap().number_inputs();
                for niid in number_input_ids {
                    let number_topo = topo.number_input(*niid).unwrap().number_graph().topology();
                    let states = object_ui_states.number_graph_object_states(*niid);
                    self.number_graph_uis.entry(*niid).or_insert_with(|| {
                        (
                            NumberGraphUiState::new(),
                            SoundNumberInputPresentation::new(number_topo, states),
                        )
                    });
                }
            }
        }
    }

    pub(super) fn number_graph_ui(
        &mut self,
        input_id: SoundNumberInputId,
    ) -> (&mut NumberGraphUiState, &mut SoundNumberInputPresentation) {
        self.number_graph_uis
            .get_mut(&input_id)
            .map(|(a, b)| (a, b))
            .unwrap()
    }
}
