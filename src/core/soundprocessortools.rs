use std::sync::Arc;

use super::{
    key::{Key, TypeErasedKey},
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{
        InputTimeNumberSource, KeyedInputNumberSource, NumberSourceHandle, NumberSourceId,
        NumberSourceOwner, ProcessorNumberSource, ProcessorTimeNumberSource, StateFunction,
    },
    soundgraphtopology::SoundGraphTopology,
    soundinput::{
        InputOptions, KeyedSoundInput, KeyedSoundInputHandle, SingleSoundInput,
        SingleSoundInputHandle, SoundInputId,
    },
    soundprocessor::{SoundProcessorData, SoundProcessorId},
    soundstate::SoundState,
    uniqueid::IdGenerator,
};

pub struct SoundProcessorTools<'a, T: SoundState> {
    processor_id: SoundProcessorId,
    data: Arc<SoundProcessorData<T>>,
    topology: &'a mut SoundGraphTopology,
    sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
    number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
}

impl<'a, T: SoundState> SoundProcessorTools<'a, T> {
    pub(super) fn new(
        id: SoundProcessorId,
        data: Arc<SoundProcessorData<T>>,
        topology: &'a mut SoundGraphTopology,
        input_idgen: &'a mut IdGenerator<SoundInputId>,
        number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    ) -> SoundProcessorTools<'a, T> {
        SoundProcessorTools {
            processor_id: id,
            data,
            topology,
            sound_input_idgen: input_idgen,
            number_source_idgen,
            number_input_idgen,
        }
    }

    pub fn add_single_sound_input(&mut self, options: InputOptions) -> SingleSoundInputHandle {
        let input_id = self.sound_input_idgen.next_id();
        let (input, handle) = SingleSoundInput::new(input_id, options);
        self.topology.add_sound_input(self.processor_id, input);
        handle
    }

    pub fn add_keyed_sound_input<K: Key, TT: SoundState>(
        &mut self,
        options: InputOptions,
    ) -> KeyedSoundInputHandle<K, TT> {
        let input_id = self.sound_input_idgen.next_id();
        let (input, handle) = KeyedSoundInput::<K, TT>::new(input_id, options);
        self.topology.add_sound_input(self.processor_id, input);
        handle
    }

    pub fn add_processor_number_source<F: StateFunction<T>>(
        &mut self,
        function: F,
    ) -> NumberSourceHandle {
        let nsid = self.number_source_idgen.next_id();
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let instance = Arc::new(ProcessorNumberSource::new(Arc::clone(&self.data), function));
        self.topology.add_number_source(nsid, instance, owner);
        NumberSourceHandle::new(nsid, owner)
    }

    pub fn add_processor_time(&mut self) -> NumberSourceHandle {
        let nsid = self.number_source_idgen.next_id();
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let instance = Arc::new(ProcessorTimeNumberSource::new(self.processor_id));
        self.topology.add_number_source(nsid, instance, owner);
        NumberSourceHandle::new(nsid, owner)
    }

    pub fn add_input_time(&mut self, input_id: SoundInputId) -> NumberSourceHandle {
        let nsid = self.number_source_idgen.next_id();
        let owner = NumberSourceOwner::SoundInput(input_id);
        let instance = Arc::new(InputTimeNumberSource::new(input_id));
        self.topology.add_number_source(nsid, instance, owner);
        NumberSourceHandle::new(nsid, owner)
    }

    pub fn add_keyed_input_number_source<K: Key, TT: SoundState, F: StateFunction<TT>>(
        &mut self,
        handle: &KeyedSoundInputHandle<K, TT>,
        f: F,
    ) -> NumberSourceHandle {
        let nsid = self.number_source_idgen.next_id();
        let owner = handle.id();
        let source = Arc::new(KeyedInputNumberSource::new(handle.clone(), f));
        self.topology
            .add_number_source(nsid, source, NumberSourceOwner::SoundInput(owner));
        NumberSourceHandle::new(nsid, NumberSourceOwner::SoundInput(owner))
    }

    pub fn add_number_input(&mut self) -> NumberInputHandle {
        let niid = self.number_input_idgen.next_id();
        let owner = NumberInputOwner::SoundProcessor(self.processor_id);
        let handle = NumberInputHandle::new(niid, owner);
        self.topology.add_number_input(handle.clone());
        handle
    }

    pub fn remove_single_sound_input(&mut self, handle: SingleSoundInputHandle) {
        self.topology.remove_sound_input(handle.id());
    }

    pub fn remove_keyed_sound_input<K: Key, TT: SoundState>(
        &mut self,
        handle: KeyedSoundInputHandle<K, TT>,
    ) {
        self.topology.remove_sound_input(handle.id());
    }

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) {
        self.topology.remove_number_input(handle.id());
    }

    pub fn remove_number_source(&mut self, handle: NumberSourceHandle) {
        self.topology.remove_number_source(handle.id());
    }

    pub(super) fn add_keyed_input_key(&mut self, input_id: SoundInputId, key: TypeErasedKey) {
        self.topology.add_sound_input_key(input_id, key);
    }

    pub(super) fn remove_keyed_input_key(&mut self, input_id: SoundInputId, key_index: usize) {
        self.topology.remove_sound_input_key(input_id, key_index);
    }
}
