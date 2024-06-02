use crate::core::sound::{
    expression::SoundExpressionId, soundgraphid::SoundGraphId, soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::lexicallayout::lexicallayout::LexicalLayoutFocus;

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
}
