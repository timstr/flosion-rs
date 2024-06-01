use eframe::egui;

use crate::core::{
    expression::expressiongraph::ExpressionGraph,
    graph::objectfactory::ObjectFactory,
    jit::server::JitClient,
    sound::{
        expression::SoundExpressionId, soundgraph::SoundGraph, soundgraphid::SoundGraphId,
        soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    expressiongraphui::ExpressionGraphUi, lexicallayout::lexicallayout::LexicalLayoutFocus,
    expressiongraphuicontext::OuterProcessorExpressionContext,
    expressiongraphuistate::ExpressionUiCollection, soundgraphuinames::SoundGraphUiNames,
    soundobjectuistate::SoundObjectUiStates, temporallayout::SoundGraphLayout,
    ui_factory::UiFactory,
};

pub(super) enum KeyboardFocusState {
    AroundSoundProcessor(SoundProcessorId),
    OnSoundProcessorName(SoundProcessorId),
    AroundSoundInput(SoundInputId),
    InsideEmptySoundInput(SoundInputId),
    AroundExpression(SoundExpressionId),
    InsideExpression(SoundExpressionId, LexicalLayoutFocus),
}

impl KeyboardFocusState {
    pub(super) fn graph_id(&self) -> SoundGraphId {
        match self {
            KeyboardFocusState::AroundSoundProcessor(spid) => (*spid).into(),
            KeyboardFocusState::OnSoundProcessorName(spid) => (*spid).into(),
            KeyboardFocusState::AroundSoundInput(siid) => (*siid).into(),
            KeyboardFocusState::InsideEmptySoundInput(siid) => (*siid).into(),
            KeyboardFocusState::AroundExpression(niid) => (*niid).into(),
            KeyboardFocusState::InsideExpression(niid, _) => (*niid).into(),
        }
    }

    pub(super) fn item_has_keyboard_focus(&self, item: SoundGraphId) -> bool {
        self.graph_id() == item
    }

    pub(super) fn expression_focus(
        &mut self,
        id: SoundExpressionId,
    ) -> Option<&mut LexicalLayoutFocus> {
        match self {
            KeyboardFocusState::InsideExpression(snid, focus) => {
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
        layout: &SoundGraphLayout,
        input: &mut egui::InputState,
    ) -> bool {
        if let KeyboardFocusState::InsideExpression(niid, _) = self {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                *self = KeyboardFocusState::AroundExpression(*niid);
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
                    if layout.is_top_level(target_spid.into())
                        && input.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                    {
                        *self = KeyboardFocusState::AroundSoundProcessor(target_spid);
                        return true;
                    }
                } else {
                    *self = KeyboardFocusState::InsideEmptySoundInput(*siid);
                }
            }
            KeyboardFocusState::AroundExpression(niid) => {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                    *self = KeyboardFocusState::InsideExpression(*niid, LexicalLayoutFocus::new());
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
        layout: &SoundGraphLayout,
        names: &SoundGraphUiNames,
        expr_graph_uis: &mut ExpressionUiCollection,
        object_factory: &ObjectFactory<ExpressionGraph>,
        ui_factory: &UiFactory<ExpressionGraphUi>,
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

            while self.handle_single_keyboard_event(soundgraph.topology(), layout, i) {}
        });

        if let KeyboardFocusState::InsideExpression(niid, ni_focus) = self {
            let (_ui_state, ui_presentation) = expr_graph_uis.get_mut(*niid).unwrap();
            let object_ui_states = object_ui_states.expression_graph_object_state_mut(*niid);
            let owner = soundgraph.topology().expression(*niid).unwrap().owner();
            let time_axis = layout.find_group(owner.into()).unwrap().time_axis;
            let outer_context = OuterProcessorExpressionContext::new(
                *niid, owner, layout, soundgraph, names, jit_client, time_axis,
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
