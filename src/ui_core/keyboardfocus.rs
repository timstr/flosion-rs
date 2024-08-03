use crate::core::sound::{
    expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
    soundprocessor::SoundProcessorId,
};

use super::{
    lexicallayout::lexicallayout::LexicalLayoutFocus,
    stackedlayout::interconnect::ProcessorInterconnect,
};

pub(super) enum KeyboardFocusState {
    AroundSoundProcessor(SoundProcessorId),
    OnSoundProcessorName(SoundProcessorId),
    AroundInterconnect(ProcessorInterconnect),
    AroundExpression(SoundExpressionId),
    InsideExpression(SoundExpressionId, LexicalLayoutFocus),
}

impl KeyboardFocusState {
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

    /// Returns true iff all graph ids referenced by the keyboard focus
    /// refer to objects that exist in the given topology
    pub(super) fn is_valid(&self, topo: &SoundGraphTopology) -> bool {
        match self {
            KeyboardFocusState::AroundSoundProcessor(spid) => topo.contains(spid),
            KeyboardFocusState::OnSoundProcessorName(spid) => topo.contains(spid),
            KeyboardFocusState::AroundInterconnect(i) => i.is_valid(topo),
            KeyboardFocusState::AroundExpression(eid) => topo.contains(eid),
            KeyboardFocusState::InsideExpression(eid, _) => topo.contains(eid),
        }
    }
}
