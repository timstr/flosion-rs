use std::sync::Arc;

use super::{
    compilednumberinput::{ArrayReadFunc, ScalarReadFunc},
    graphobject::ObjectInitialization,
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{
        ArrayInputNumberSource, ArrayProcessorNumberSource, InputTimeNumberSource,
        NumberSourceHandle, NumberSourceId, NumberSourceOwner, NumberVisibility,
        ProcessorTimeNumberSource, PureNumberSource, PureNumberSourceHandle,
        PureNumberSourceWithId, ScalarInputNumberSource, ScalarProcessorNumberSource,
    },
    numbersourcetools::NumberSourceTools,
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
    pub(crate) fn new(
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

    pub fn add_input_scalar_number_source(
        &mut self,
        input_id: SoundInputId,
        function: ScalarReadFunc,
        visibility: NumberVisibility,
    ) -> NumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ScalarInputNumberSource::new(input_id, function));
        let owner = NumberSourceOwner::SoundInput(input_id);
        let data = NumberSourceData::new(id, instance, owner, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        NumberSourceHandle::new(id, visibility)
    }

    pub fn add_input_array_number_source(
        &mut self,
        input_id: SoundInputId,
        function: ArrayReadFunc,
        visibility: NumberVisibility,
    ) -> NumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ArrayInputNumberSource::new(input_id, function));
        let owner = NumberSourceOwner::SoundInput(input_id);
        let data = NumberSourceData::new(id, instance, owner, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        NumberSourceHandle::new(id, visibility)
    }

    pub fn add_processor_scalar_number_source(
        &mut self,
        function: ScalarReadFunc,
        visibility: NumberVisibility,
    ) -> NumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ScalarProcessorNumberSource::new(
            self.processor_id,
            function,
        ));
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let data = NumberSourceData::new(id, instance, owner, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        NumberSourceHandle::new(id, visibility)
    }

    pub fn add_processor_array_number_source(
        &mut self,
        function: ArrayReadFunc,
        visibility: NumberVisibility,
    ) -> NumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ArrayProcessorNumberSource::new(self.processor_id, function));
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let data = NumberSourceData::new(id, instance, owner, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        NumberSourceHandle::new(id, visibility)
    }

    pub fn add_number_input(&mut self, default_value: f32) -> NumberInputHandle {
        let id = self.number_input_idgen.next_id();
        let target = None;
        let owner = NumberInputOwner::SoundProcessor(self.processor_id);
        let visibility = NumberVisibility::Public;
        let data = NumberInputData::new(id, target, owner, default_value, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberInput(data));
        NumberInputHandle::new(id, owner, visibility)
    }

    pub fn add_processor_time(&mut self, visibility: NumberVisibility) -> NumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(ProcessorTimeNumberSource::new(self.processor_id));
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let data = NumberSourceData::new(id, instance, owner, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        NumberSourceHandle::new(id, visibility)
    }

    pub fn add_input_time(
        &mut self,
        input_id: SoundInputId,
        visibility: NumberVisibility,
    ) -> NumberSourceHandle {
        let id = self.number_source_idgen.next_id();
        let instance = Arc::new(InputTimeNumberSource::new(input_id));
        let owner = NumberSourceOwner::SoundInput(input_id);
        let data = NumberSourceData::new(id, instance, owner, visibility);
        self.edit_queue.push(SoundGraphEdit::AddNumberSource(data));
        NumberSourceHandle::new(id, visibility)
    }

    pub fn add_derived_processor_number_source<T: PureNumberSource>(
        &mut self,
        visibility: NumberVisibility,
    ) -> Result<PureNumberSourceHandle<T>, ()> {
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        self.add_derived_number_source::<T>(owner, visibility)
    }

    pub fn add_derived_input_number_source<T: PureNumberSource>(
        &mut self,
        input_id: SoundInputId,
        visibility: NumberVisibility,
    ) -> Result<PureNumberSourceHandle<T>, ()> {
        let owner = NumberSourceOwner::SoundInput(input_id);
        self.add_derived_number_source::<T>(owner, visibility)
    }

    fn add_derived_number_source<T: PureNumberSource>(
        &mut self,
        owner: NumberSourceOwner,
        visibility: NumberVisibility,
    ) -> Result<PureNumberSourceHandle<T>, ()> {
        let nsid = self.number_source_idgen.next_id();
        let start_of_queue = self.edit_queue.len();
        let instance;
        {
            let ns_input_visibility = NumberVisibility::Private;
            let ns_tools = NumberSourceTools::new(
                nsid,
                self.number_input_idgen,
                self.edit_queue,
                ns_input_visibility,
            );
            let s = T::new(ns_tools, ObjectInitialization::Default)?;
            instance = Arc::new(PureNumberSourceWithId::new(s, nsid, owner, visibility));
        }
        let instance2 = Arc::clone(&instance);
        let data = NumberSourceData::new(nsid, instance2, owner, visibility);
        self.edit_queue
            .insert(start_of_queue, SoundGraphEdit::AddNumberSource(data));

        Ok(PureNumberSourceHandle::new(instance))
    }

    pub fn connect_number_input(&mut self, input_id: NumberInputId, source_id: NumberSourceId) {
        self.edit_queue
            .push(SoundGraphEdit::ConnectNumberInput(input_id, source_id));
    }

    pub(super) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }
}
