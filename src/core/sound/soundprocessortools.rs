use std::sync::Arc;

use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    uniqueid::IdGenerator,
};

use super::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgraphdata::{SoundInputData, SoundNumberInputData, SoundNumberSourceData},
    soundgraphedit::SoundGraphEdit,
    soundinput::{InputOptions, SoundInputId},
    soundnumberinput::{SoundNumberInputHandle, SoundNumberInputId},
    soundnumbersource::{
        ArrayInputNumberSource, ArrayProcessorNumberSource, ProcessorLocalArrayNumberSource,
        ScalarInputNumberSource, ScalarProcessorNumberSource, SoundNumberSourceHandle,
        SoundNumberSourceId, SoundNumberSourceOwner,
    },
    soundprocessor::SoundProcessorId,
};

pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,
    sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
    number_input_idgen: &'a mut IdGenerator<SoundNumberInputId>,
    number_source_idgen: &'a mut IdGenerator<SoundNumberSourceId>,
    edit_queue: &'a mut Vec<SoundGraphEdit>,
}

impl<'a> SoundProcessorTools<'a> {
    pub(crate) fn new(
        id: SoundProcessorId,
        sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
        number_input_idgen: &'a mut IdGenerator<SoundNumberInputId>,
        number_source_idgen: &'a mut IdGenerator<SoundNumberSourceId>,
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
        let (add_time, time_nsid) = SoundGraphEdit::add_input_time(id, self.number_source_idgen);
        let owner = self.processor_id;
        let data = SoundInputData::new(id, options, num_keys, owner, time_nsid);
        self.edit_queue
            .push(SoundGraphEdit::Sound(SoundEdit::AddSoundInput(data)));
        self.edit_queue.push(add_time);
        id
    }

    pub(super) fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        self.edit_queue
            .push(SoundGraphEdit::Sound(SoundEdit::RemoveSoundInput(
                input_id, owner,
            )));
    }

    pub fn add_input_scalar_number_source(
        &mut self,
        input_id: SoundInputId,
        function: ScalarReadFunc,
    ) -> SoundNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ScalarInputNumberSource::new(input_id, function));
        let owner = SoundNumberSourceOwner::SoundInput(input_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        self.edit_queue
            .push(SoundNumberEdit::AddNumberSource(data).into());
        SoundNumberSourceHandle::new(id)
    }

    pub fn add_input_array_number_source(
        &mut self,
        input_id: SoundInputId,
        function: ArrayReadFunc,
    ) -> SoundNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ArrayInputNumberSource::new(input_id, function));
        let owner = SoundNumberSourceOwner::SoundInput(input_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        self.edit_queue
            .push(SoundNumberEdit::AddNumberSource(data).into());
        SoundNumberSourceHandle::new(id)
    }

    pub fn add_processor_scalar_number_source(
        &mut self,
        function: ScalarReadFunc,
    ) -> SoundNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ScalarProcessorNumberSource::new(
            self.processor_id,
            function,
        ));
        let owner = SoundNumberSourceOwner::SoundProcessor(self.processor_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        self.edit_queue
            .push(SoundNumberEdit::AddNumberSource(data).into());
        SoundNumberSourceHandle::new(id)
    }

    pub fn add_processor_array_number_source(
        &mut self,
        function: ArrayReadFunc,
    ) -> SoundNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ArrayProcessorNumberSource::new(self.processor_id, function));
        let owner = SoundNumberSourceOwner::SoundProcessor(self.processor_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        self.edit_queue
            .push(SoundNumberEdit::AddNumberSource(data).into());
        SoundNumberSourceHandle::new(id)
    }

    pub fn add_local_array_number_source(&mut self) -> SoundNumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ProcessorLocalArrayNumberSource::new(id, self.processor_id));
        let owner = SoundNumberSourceOwner::SoundProcessor(self.processor_id);
        let data = SoundNumberSourceData::new(id, instance, owner);
        self.edit_queue
            .push(SoundNumberEdit::AddNumberSource(data).into());
        SoundNumberSourceHandle::new(id)
    }

    pub fn add_number_input(&mut self, default_value: f32) -> SoundNumberInputHandle {
        let id = self.number_input_idgen.next_id();
        let owner = self.processor_id;
        let data = SoundNumberInputData::new(id, owner, default_value);
        self.edit_queue
            .push(SoundNumberEdit::AddNumberInput(data).into());
        SoundNumberInputHandle::new(id, owner)
    }

    pub(super) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }
}
