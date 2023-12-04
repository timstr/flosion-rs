use std::sync::Arc;

use crate::core::uniqueid::IdGenerator;

use super::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgraphdata::SoundNumberSourceData,
    soundgrapherror::SoundError,
    soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId,
    soundnumbersource::{
        InputTimeNumberSource, ProcessorTimeNumberSource, SoundNumberSourceId,
        SoundNumberSourceOwner,
    },
    soundprocessor::SoundProcessorId,
};

pub(crate) enum SoundGraphEdit {
    Sound(SoundEdit),
    Number(SoundNumberEdit),
}

impl SoundGraphEdit {
    pub(super) fn add_processor_time(
        processor_id: SoundProcessorId,
        number_source_idgen: &mut IdGenerator<SoundNumberSourceId>,
    ) -> (SoundGraphEdit, SoundNumberSourceId) {
        let id = number_source_idgen.next_id();
        let instance = Arc::new(ProcessorTimeNumberSource::new(processor_id));
        let owner = SoundNumberSourceOwner::SoundProcessor(processor_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        (SoundNumberEdit::AddNumberSource(data).into(), id)
    }

    pub(super) fn add_input_time(
        input_id: SoundInputId,
        number_source_idgen: &mut IdGenerator<SoundNumberSourceId>,
    ) -> (SoundGraphEdit, SoundNumberSourceId) {
        let id = number_source_idgen.next_id();
        let instance = Arc::new(InputTimeNumberSource::new(input_id));
        let owner = SoundNumberSourceOwner::SoundInput(input_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        (SoundNumberEdit::AddNumberSource(data).into(), id)
    }

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

impl From<SoundEdit> for SoundGraphEdit {
    fn from(value: SoundEdit) -> Self {
        SoundGraphEdit::Sound(value)
    }
}

impl From<SoundNumberEdit> for SoundGraphEdit {
    fn from(value: SoundNumberEdit) -> Self {
        SoundGraphEdit::Number(value)
    }
}
