use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::sound::{
    soundgraphid::{SoundGraphId, SoundObjectId},
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::find_sound_error,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::{
    keyboardfocus::KeyboardFocusState,
    numbergraphuistate::{NumberGraphUiState, SoundNumberInputUiCollection},
    object_positions::ObjectPositions,
    soundgraphuinames::SoundGraphUiNames,
    soundnumberinputui::SoundNumberInputPresentation,
    soundobjectuistate::SoundObjectUiStates,
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
    // NOTE to self: remove SoundGraphLayout (previously TemporalLayout) from
    // here altogether. Store SoundGraphLayout in FlosionApp directly
    // and move most or all of this into SoundGraphLayout
    mode: UiMode,
    pending_drag: Option<PendingProcessorDrag>,
    number_input_uis: SoundNumberInputUiCollection,
    names: SoundGraphUiNames,
}

impl SoundGraphUiState {
    pub(super) fn new() -> SoundGraphUiState {
        SoundGraphUiState {
            object_positions: ObjectPositions::new(),
            mode: UiMode::Passive,
            pending_drag: None,
            number_input_uis: SoundNumberInputUiCollection::new(),
            names: SoundGraphUiNames::new(),
        }
    }

    pub(super) fn object_positions(&self) -> &ObjectPositions {
        &self.object_positions
    }

    pub(super) fn object_positions_mut(&mut self) -> &mut ObjectPositions {
        &mut self.object_positions
    }

    fn update_mode_from_selection(&mut self) {
        if let UiMode::Selecting(object_ids) = &mut self.mode {
            match object_ids.len() {
                0 => self.mode = UiMode::Passive,
                1 => {
                    self.mode = UiMode::UsingKeyboardNav(match object_ids.iter().next().unwrap() {
                        SoundObjectId::Sound(spid) => {
                            KeyboardFocusState::AroundSoundProcessor(*spid)
                        }
                    })
                }
                _ => (),
            }
        }
    }

    pub(super) fn stop_selecting(&mut self) {
        match self.mode {
            UiMode::Selecting(_) => self.mode = UiMode::Passive,
            _ => (),
        }
    }

    pub(super) fn set_selection(&mut self, object_ids: HashSet<SoundObjectId>) {
        self.mode = UiMode::Selecting(object_ids);
        self.update_mode_from_selection();
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
        self.update_mode_from_selection();
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
                topo_disconnected.disconnect_number_input(niid, nsid)
            }
            topo_disconnected
                .disconnect_sound_input(si_data.id())
                .unwrap();
        }

        let topo_disconnected = topo_disconnected;

        debug_assert_eq!(find_sound_error(&topo_disconnected), None);

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
                .connect_sound_input(si_data.id(), processor_id)
                .unwrap();
            if find_sound_error(&topo_reconnected).is_none() {
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

    pub(super) fn cleanup(
        &mut self,
        // TODO: remove this hashset completely here and elsewhere, refer to topology only
        remaining_ids: &HashSet<SoundGraphId>,
        topo: &SoundGraphTopology,
        object_ui_states: &SoundObjectUiStates,
    ) {
        self.object_positions.retain(remaining_ids);

        match &mut self.mode {
            UiMode::Selecting(s) => {
                s.retain(|id| remaining_ids.contains(&(*id).into()));
                if s.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Passive => (),
            UiMode::UsingKeyboardNav(kbd_focus) => {
                if !remaining_ids.contains(&kbd_focus.graph_id()) {
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

        self.number_input_uis.cleanup(topo, object_ui_states);

        self.names.regenerate(topo);
    }

    pub(super) fn effective_selection(&self) -> HashSet<SoundObjectId> {
        // TODO: this is silly, don't clone the selection.
        match &self.mode {
            UiMode::Selecting(s) => s.clone(),
            UiMode::UsingKeyboardNav(kbd) => match kbd {
                KeyboardFocusState::AroundSoundProcessor(spid) => {
                    let mut h = HashSet::new();
                    h.insert((*spid).into());
                    h
                }
                _ => HashSet::new(),
            },
            _ => HashSet::new(),
        }
    }

    pub(super) fn is_object_selected(&self, object_id: SoundObjectId) -> bool {
        match &self.mode {
            UiMode::Selecting(s) => s.contains(&object_id),
            _ => false,
        }
    }

    pub(super) fn keyboard_focus(&self) -> Option<&KeyboardFocusState> {
        match &self.mode {
            UiMode::UsingKeyboardNav(kbd) => Some(kbd),
            _ => None,
        }
    }

    pub(super) fn is_item_focused(&self, id: SoundGraphId) -> bool {
        match &self.mode {
            UiMode::UsingKeyboardNav(kbd) => kbd.item_has_keyboard_focus(id),
            _ => false,
        }
    }

    pub(super) fn item_with_keyboard_focus(&self) -> Option<SoundGraphId> {
        match &self.mode {
            UiMode::UsingKeyboardNav(kbd) => Some(kbd.graph_id()),
            _ => None,
        }
    }

    pub(super) fn set_keyboard_focus(&mut self, focus: KeyboardFocusState) {
        self.mode = UiMode::UsingKeyboardNav(focus);
    }

    pub(super) fn item_has_keyboard_focus(&self, id: SoundGraphId) -> bool {
        match &self.mode {
            UiMode::UsingKeyboardNav(k) => k.item_has_keyboard_focus(id),
            _ => false,
        }
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
                    let states = object_ui_states.number_graph_object_state(*niid);
                    self.number_input_uis.set_ui_data(
                        *niid,
                        NumberGraphUiState::new(),
                        SoundNumberInputPresentation::new(number_topo, states),
                    );
                }
            }
        }
    }

    pub(crate) fn names(&self) -> &SoundGraphUiNames {
        &self.names
    }

    pub(crate) fn names_mut(&mut self) -> &mut SoundGraphUiNames {
        &mut self.names
    }
}
