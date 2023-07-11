use crate::core::sound::{
    soundgraphid::{SoundGraphId, SoundObjectId},
    soundprocessor::SoundProcessorId,
};

#[derive(Copy, Clone)]
pub(super) enum KeyboardFocusState {
    SoundProcessor(SoundProcessorId),
}

impl KeyboardFocusState {
    pub(super) fn as_graph_id(&self) -> SoundGraphId {
        match self {
            KeyboardFocusState::SoundProcessor(i) => (*i).into(),
        }
    }

    pub(super) fn object_has_keyboard_focus(&self, object: SoundObjectId) -> bool {
        match (object, self) {
            (SoundObjectId::Sound(spid1), KeyboardFocusState::SoundProcessor(spid2)) => {
                spid1 == *spid2
            }
        }
    }
}
