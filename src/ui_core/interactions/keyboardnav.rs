use crate::{
    core::sound::{
        expression::SoundExpressionId, soundgraphtopology::SoundGraphTopology,
        soundprocessor::SoundProcessorId,
    },
    ui_core::{
        lexicallayout::lexicallayout::LexicalLayoutFocus,
        stackedlayout::interconnect::{InputSocket, ProcessorPlug},
    },
};

pub(crate) enum KeyboardNavInteraction {
    AroundSoundProcessor(SoundProcessorId),
    OnSoundProcessorName(SoundProcessorId),
    AroundProcessorPlug(ProcessorPlug),
    AroundInputSocket(InputSocket),
    AroundExpression(SoundExpressionId),
    InsideExpression(SoundExpressionId, LexicalLayoutFocus),
}

impl KeyboardNavInteraction {
    pub(crate) fn expression_focus(
        &mut self,
        id: SoundExpressionId,
    ) -> Option<&mut LexicalLayoutFocus> {
        match self {
            KeyboardNavInteraction::InsideExpression(snid, focus) => {
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
    pub(crate) fn is_valid(&self, topo: &SoundGraphTopology) -> bool {
        match self {
            KeyboardNavInteraction::AroundSoundProcessor(spid) => topo.contains(spid),
            KeyboardNavInteraction::OnSoundProcessorName(spid) => topo.contains(spid),
            KeyboardNavInteraction::AroundProcessorPlug(p) => topo.contains(p.processor),
            KeyboardNavInteraction::AroundInputSocket(s) => topo.contains(s.input),
            KeyboardNavInteraction::AroundExpression(eid) => topo.contains(eid),
            KeyboardNavInteraction::InsideExpression(eid, _) => topo.contains(eid),
        }
    }
}
