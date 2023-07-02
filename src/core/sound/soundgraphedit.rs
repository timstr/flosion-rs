use super::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgrapherror::SoundError,
    soundgraphtopology::SoundGraphTopology,
};

pub(crate) enum SoundGraphEdit {
    Sound(SoundEdit),
    Number(SoundNumberEdit),
}

impl SoundGraphEdit {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            SoundGraphEdit::Sound(e) => e.name(),
            SoundGraphEdit::Number(e) => e.name(),
        }
    }

    pub(crate) fn check_preconditions(&self, topology: &SoundGraphTopology) -> Option<SoundError> {
        match self {
            SoundGraphEdit::Sound(e) => e.check_preconditions(topology),
            SoundGraphEdit::Number(e) => e.check_preconditions(topology),
        }
    }
}
