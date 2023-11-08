use eframe::egui;

use crate::core::{
    graph::objectfactory::ObjectFactory,
    number::numbergraph::NumberGraph,
    sound::{
        soundgraph::SoundGraph, soundgraphid::SoundGraphId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundnumberinput::SoundNumberInputId,
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    lexicallayout::lexicallayout::LexicalLayoutFocus,
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::OuterSoundNumberInputContext,
    numbergraphuistate::SoundNumberInputUiCollection,
    object_positions::ObjectPositions,
    soundgraphui::SoundGraphUi,
    soundgraphuinames::SoundGraphUiNames,
    soundgraphuistate::SoundGraphUiState,
    soundinputsummon::{build_summon_widget_for_sound_input, SoundInputSummonValue},
    soundobjectuistate::SoundObjectUiStates,
    summon_widget::SummonWidgetState,
    temporallayout::TemporalLayout,
    ui_factory::UiFactory,
};

pub(super) enum KeyboardFocusState {
    AroundSoundProcessor(SoundProcessorId),
    AroundSoundInput(SoundInputId),
    InsideEmptySoundInput(SoundInputId, SummonWidgetState<SoundInputSummonValue>),
    AroundSoundNumberInput(SoundNumberInputId),
    InsideSoundNumberInput(SoundNumberInputId, LexicalLayoutFocus),
}

impl KeyboardFocusState {
    pub(super) fn graph_id(&self) -> SoundGraphId {
        match self {
            KeyboardFocusState::AroundSoundProcessor(spid) => (*spid).into(),
            KeyboardFocusState::AroundSoundInput(siid) => (*siid).into(),
            KeyboardFocusState::InsideEmptySoundInput(siid, _) => (*siid).into(),
            KeyboardFocusState::AroundSoundNumberInput(niid) => (*niid).into(),
            KeyboardFocusState::InsideSoundNumberInput(niid, _) => (*niid).into(),
        }
    }

    pub(super) fn item_has_keyboard_focus(&self, item: SoundGraphId) -> bool {
        self.graph_id() == item
    }

    pub(super) fn sound_number_input_focus(
        &mut self,
        id: SoundNumberInputId,
    ) -> Option<&mut LexicalLayoutFocus> {
        match self {
            KeyboardFocusState::InsideSoundNumberInput(snid, focus) => {
                if *snid == id {
                    Some(focus)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_single_keyboard_event(
        &mut self,
        topology: &SoundGraphTopology,
        temporal_layout: &TemporalLayout,
        object_positions: &ObjectPositions,
        ui_factory: &UiFactory<SoundGraphUi>,
        input: &mut egui::InputState,
    ) -> bool {
        if let KeyboardFocusState::InsideSoundNumberInput(niid, _) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundSoundNumberInput(*niid);
                return true;
            }
            return false;
        } else if let KeyboardFocusState::InsideEmptySoundInput(siid, _) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundSoundInput(*siid);
                return true;
            }
        } else {
            let graph_id = self.graph_id();
            let root_spid = temporal_layout.find_root_processor(graph_id, topology);

            let items = temporal_layout.get_stack_items(root_spid, topology);

            let mut index = items.iter().position(|i| *i == graph_id).unwrap();
            let mut did_anything = false;

            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                index = index.saturating_sub(1);
                did_anything = true;
            }

            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                index = (index + 1).min(items.len() - 1);
                did_anything = true;
            }

            if did_anything {
                let new_item = items[index];
                *self = match new_item {
                    SoundGraphId::SoundInput(siid) => KeyboardFocusState::AroundSoundInput(siid),
                    SoundGraphId::SoundProcessor(spid) => {
                        KeyboardFocusState::AroundSoundProcessor(spid)
                    }
                    SoundGraphId::SoundNumberInput(nsid) => {
                        KeyboardFocusState::AroundSoundNumberInput(nsid)
                    }
                    SoundGraphId::SoundNumberSource(_) => panic!(),
                };
                return true;
            }
        }

        if let KeyboardFocusState::AroundSoundNumberInput(niid) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                *self =
                    KeyboardFocusState::InsideSoundNumberInput(*niid, LexicalLayoutFocus::new());
                return true;
            }
        }

        if let KeyboardFocusState::AroundSoundInput(siid) = self {
            if let Some(target_spid) = topology.sound_input(*siid).unwrap().target() {
                if temporal_layout.is_top_level(target_spid.into())
                    && input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                {
                    *self = KeyboardFocusState::AroundSoundProcessor(target_spid);
                    return true;
                }
            } else {
                // input is empty
                if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    let position = object_positions
                        .get_sound_input_location(*siid)
                        .unwrap()
                        .rect()
                        .left_bottom();
                    *self = KeyboardFocusState::InsideEmptySoundInput(
                        *siid,
                        build_summon_widget_for_sound_input(position, ui_factory),
                    );
                }
            }
        }

        false
    }

    pub(super) fn handle_keyboard_focus(
        &mut self,
        ui: &egui::Ui,
        soundgraph: &mut SoundGraph,
        temporal_layout: &TemporalLayout,
        names: &SoundGraphUiNames,
        object_positions: &ObjectPositions,
        sound_ui_factory: &UiFactory<SoundGraphUi>,
        number_graph_uis: &mut SoundNumberInputUiCollection,
        number_object_factory: &ObjectFactory<NumberGraph>,
        number_ui_factory: &UiFactory<NumberGraphUi>,
        sound_graph_ui_state: &SoundGraphUiState,
        object_ui_states: &mut SoundObjectUiStates,
    ) {
        ui.input_mut(|i| {
            //  preemptively avoid some unnecessary computation.
            // These keys will be consumed in handle_single_keyboard_event
            if !(i.key_pressed(egui::Key::ArrowUp)
                || i.key_pressed(egui::Key::ArrowDown)
                || i.key_pressed(egui::Key::Enter)
                || i.key_pressed(egui::Key::Escape))
            {
                return;
            }

            while self.handle_single_keyboard_event(
                soundgraph.topology(),
                temporal_layout,
                object_positions,
                sound_ui_factory,
                i,
            ) {}
        });

        if let KeyboardFocusState::InsideSoundNumberInput(niid, ni_focus) = self {
            let (_ui_state, ui_presentation) = number_graph_uis.get_mut(*niid).unwrap();
            let number_object_ui_states = object_ui_states.number_graph_object_state(*niid);
            let owner = soundgraph.topology().number_input(*niid).unwrap().owner();
            let outer_context = OuterSoundNumberInputContext::new(
                *niid,
                owner,
                temporal_layout,
                soundgraph,
                names,
                sound_graph_ui_state,
                object_ui_states,
            );
            ui_presentation.handle_keypress(
                ui,
                ni_focus,
                number_object_factory,
                number_ui_factory,
                &mut number_object_ui_states.borrow_mut(),
                &mut outer_context.into(),
            );
        }
    }
}
