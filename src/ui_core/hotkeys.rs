use crate::core::{
    graphobject::{GraphId, ObjectId},
    numbersource::NumberSourceId,
    soundprocessor::SoundProcessorId,
};

#[derive(Copy, Clone)]
pub(super) enum KeyboardFocusState {
    SoundProcessor(SoundProcessorId),
    NumberSource(NumberSourceId),
}

impl KeyboardFocusState {
    pub(super) fn as_graph_id(&self) -> GraphId {
        match self {
            KeyboardFocusState::SoundProcessor(i) => (*i).into(),
            KeyboardFocusState::NumberSource(i) => (*i).into(),
        }
    }

    pub(super) fn object_has_keyboard_focus(&self, object: ObjectId) -> bool {
        match (object, self) {
            (ObjectId::Sound(spid1), KeyboardFocusState::SoundProcessor(spid2)) => spid1 == *spid2,
            (ObjectId::Number(nsid1), KeyboardFocusState::NumberSource(nsid2)) => nsid1 == *nsid2,
            _ => false,
        }
    }
}
