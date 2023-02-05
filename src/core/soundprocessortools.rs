use std::sync::Arc;

use super::{
    compilednumberinput::ArrayReadFunc,
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{
        NumberSource, NumberSourceId, NumberSourceOwner, ProcessorNumberSource,
        ProcessorTimeNumberSource, StateNumberSourceHandle,
    },
    soundgraphdata::{NumberInputData, NumberSourceData, SoundInputData},
    soundgraphedit::SoundGraphEdit,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
    uniqueid::IdGenerator,
};

pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,
    sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
    edit_queue: &'a mut Vec<SoundGraphEdit>,
}

impl<'a> SoundProcessorTools<'a> {
    pub(super) fn new(
        id: SoundProcessorId,
        sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
        number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
        edit_queue: &'a mut Vec<SoundGraphEdit>,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
            sound_input_idgen,
            number_input_idgen,
            number_source_idgen,
            edit_queue,
        }
    }

    pub(super) fn add_sound_input(
        &mut self,
        options: InputOptions,
        num_keys: usize,
    ) -> SoundInputId {
        let id = self.sound_input_idgen.next_id();
        let owner = self.processor_id;
        let data = SoundInputData::new(id, options, num_keys, owner);
        self.edit_queue.push(SoundGraphEdit::AddSoundInput(data));
        id
    }

    pub(super) fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        self.edit_queue
            .push(SoundGraphEdit::RemoveSoundInput(input_id, owner));
    }

    pub fn add_processor_number_source(
        &mut self,
        function: ArrayReadFunc,
    ) -> StateNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ProcessorNumberSource::new(self.processor_id, function));
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let data = NumberSourceData::new(id, instance, owner);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        StateNumberSourceHandle::new(id)
    }

    pub(super) fn add_input_number_source(
        &mut self,
        input_id: SoundInputId,
        source: Arc<dyn NumberSource>,
    ) -> StateNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let owner = NumberSourceOwner::SoundInput(input_id);
        let data = NumberSourceData::new(id, source, owner);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        StateNumberSourceHandle::new(id)
    }

    pub fn add_number_input(&mut self, default_value: f32) -> NumberInputHandle {
        let id = self.number_input_idgen.next_id();
        let target = None;
        let owner = NumberInputOwner::SoundProcessor(self.processor_id);
        let data = NumberInputData::new(id, target, owner, default_value);
        self.edit_queue.push(SoundGraphEdit::AddNumberInput(data));
        NumberInputHandle::new(id, owner)
    }

    pub fn add_processor_time(&mut self) -> StateNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ProcessorTimeNumberSource::new(self.processor_id));
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let data = NumberSourceData::new(id, instance, owner);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        StateNumberSourceHandle::new(id)
    }

    pub(super) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }
}
