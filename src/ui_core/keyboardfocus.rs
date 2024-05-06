use eframe::egui;

use crate::core::{
    graph::objectfactory::ObjectFactory,
    jit::server::JitClient,
    number::numbergraph::NumberGraph,
    sound::{
        soundgraph::SoundGraph, soundgraphid::SoundGraphId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundnumberinput::SoundNumberInputId,
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    lexicallayout::lexicallayout::LexicalLayoutFocus, numbergraphui::NumberGraphUi,
    numbergraphuicontext::OuterSoundNumberInputContext,
    numbergraphuistate::SoundNumberInputUiCollection, soundgraphuinames::SoundGraphUiNames,
    soundobjectuistate::SoundObjectUiStates, temporallayout::TemporalLayout, ui_factory::UiFactory,
};

pub(super) enum KeyboardFocusState {
    AroundSoundProcessor(SoundProcessorId),
    OnSoundProcessorName(SoundProcessorId),
    AroundSoundInput(SoundInputId),
    InsideEmptySoundInput(SoundInputId),
    AroundSoundNumberInput(SoundNumberInputId),
    InsideSoundNumberInput(SoundNumberInputId, LexicalLayoutFocus),
}

impl KeyboardFocusState {
    pub(super) fn graph_id(&self) -> SoundGraphId {
        match self {
            KeyboardFocusState::AroundSoundProcessor(spid) => (*spid).into(),
            KeyboardFocusState::OnSoundProcessorName(spid) => (*spid).into(),
            KeyboardFocusState::AroundSoundInput(siid) => (*siid).into(),
            KeyboardFocusState::InsideEmptySoundInput(siid) => (*siid).into(),
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
        input: &mut egui::InputState,
    ) -> bool {
        if let KeyboardFocusState::InsideSoundNumberInput(niid, _) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundSoundNumberInput(*niid);
                return true;
            }
            return false;
        } else if let KeyboardFocusState::InsideEmptySoundInput(siid) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundSoundInput(*siid);
                return true;
            }
        } else if let KeyboardFocusState::OnSoundProcessorName(spid) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundSoundProcessor(*spid);
                return true;
            }
        }

        match self {
            KeyboardFocusState::AroundSoundProcessor(spid) => {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    *self = KeyboardFocusState::OnSoundProcessorName(*spid);
                }
            }
            KeyboardFocusState::AroundSoundInput(siid) => {
                if let Some(target_spid) = topology.sound_input(*siid).unwrap().target() {
                    if temporal_layout.is_top_level(target_spid.into())
                        && input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    {
                        *self = KeyboardFocusState::AroundSoundProcessor(target_spid);
                        return true;
                    }
                } else {
                    *self = KeyboardFocusState::InsideEmptySoundInput(*siid);
                }
            }
            KeyboardFocusState::AroundSoundNumberInput(niid) => {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    *self = KeyboardFocusState::InsideSoundNumberInput(
                        *niid,
                        LexicalLayoutFocus::new(),
                    );
                    return true;
                }
            }
            _ => (),
        }

        false
    }

    pub(super) fn handle_keyboard_focus(
        &mut self,
        ui: &egui::Ui,
        soundgraph: &mut SoundGraph,
        temporal_layout: &TemporalLayout,
        names: &SoundGraphUiNames,
        number_graph_uis: &mut SoundNumberInputUiCollection,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut SoundObjectUiStates,
        jit_client: &JitClient,
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

            while self.handle_single_keyboard_event(soundgraph.topology(), temporal_layout, i) {}
        });

        if let KeyboardFocusState::InsideSoundNumberInput(niid, ni_focus) = self {
            let (_ui_state, ui_presentation) = number_graph_uis.get_mut(*niid).unwrap();
            let object_ui_states = object_ui_states.number_graph_object_state_mut(*niid);
            let owner = soundgraph.topology().number_input(*niid).unwrap().owner();
            let time_axis = temporal_layout
                .find_layout((*niid).into(), soundgraph.topology())
                .unwrap()
                .time_axis;
            let outer_context = OuterSoundNumberInputContext::new(
                *niid,
                owner,
                temporal_layout,
                soundgraph,
                names,
                jit_client,
                time_axis,
            );
            ui_presentation.handle_keypress(
                ui,
                ni_focus,
                object_factory,
                ui_factory,
                object_ui_states,
                &mut outer_context.into(),
            );
        }
    }
}
