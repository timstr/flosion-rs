use std::sync::{mpsc::Sender, Arc};

use super::{
    key::{Key, TypeErasedKey},
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{
        KeyedInputNumberSource, NumberSourceHandle, NumberSourceId, NumberSourceOwner,
        ProcessorNumberSource, StateFunction,
    },
    resultfuture::ResultFuture,
    soundengine::SoundEngineMessage,
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
    message_queue: Vec<SoundEngineMessage>,
    sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
    number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
}

impl<'a, T: SoundState> SoundProcessorTools<'a, T> {
    pub(super) fn new(
        id: SoundProcessorId,
        data: Arc<SoundProcessorData<T>>,
        input_idgen: &'a mut IdGenerator<SoundInputId>,
        number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    ) -> SoundProcessorTools<'a, T> {
        SoundProcessorTools {
            processor_id: id,
            data,
            message_queue: Vec::new(),
            sound_input_idgen: input_idgen,
            number_source_idgen,
            number_input_idgen,
        }
    }

    // TODO: return the handle only
    pub fn add_single_sound_input(
        &mut self,
        options: InputOptions,
    ) -> (SingleSoundInputHandle, ResultFuture<(), ()>) {
        let input_id = self.sound_input_idgen.next_id();
        let (input, handle) = SingleSoundInput::new(input_id, options);
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue.push(SoundEngineMessage::AddSoundInput {
            input,
            owner: self.processor_id,
            result: outbound_result,
        });
        (handle, result_future)
    }

    // TODO: return the handle only
    pub fn add_keyed_sound_input<K: Key, TT: SoundState>(
        &mut self,
        options: InputOptions,
    ) -> (KeyedSoundInputHandle<K, TT>, ResultFuture<(), ()>) {
        let input_id = self.sound_input_idgen.next_id();
        let (input, handle) = KeyedSoundInput::<K, TT>::new(input_id, options);
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue.push(SoundEngineMessage::AddSoundInput {
            input,
            owner: self.processor_id,
            result: outbound_result,
        });
        (handle, result_future)
    }

    // TODO: return the handle only
    pub fn add_processor_number_source<F: StateFunction<T>>(
        &mut self,
        function: F,
    ) -> (NumberSourceHandle, ResultFuture<(), ()>) {
        let nsid = self.number_source_idgen.next_id();
        let owner = NumberSourceOwner::SoundProcessor(self.processor_id);
        let instance = Arc::new(ProcessorNumberSource::new(Arc::clone(&self.data), function));
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::AddNumberSource {
                id: nsid,
                source: instance,
                owner,
                result: outbound_result,
            });
        (
            NumberSourceHandle::new(nsid, NumberSourceOwner::SoundProcessor(self.processor_id)),
            result_future,
        )
    }

    // TODO: return the handle only
    // pub fn add_single_input_number_source<F: StateFunction<EmptyState>>(
    //     &mut self,
    //     handle: &SingleSoundInputHandle,
    //     f: F,
    // ) -> (NumberSourceHandle, ResultFuture<(), ()>) {
    //     let nsid = self.number_source_idgen.next_id();
    //     let owner = handle.id();
    //     let source = Arc::new(SingleInputNumberSource::new(handle.clone(), f));
    //     let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
    //     self.message_queue
    //         .push(SoundEngineMessage::AddNumberSource {
    //             id: nsid,
    //             owner: NumberSourceOwner::SoundInput(owner),
    //             source,
    //             result: outbound_result,
    //         });
    //     (
    //         NumberSourceHandle::new(nsid, NumberSourceOwner::SoundInput(owner)),
    //         result_future,
    //     )
    // }

    // TODO: return the handle only
    pub fn add_keyed_input_number_source<K: Key, TT: SoundState, F: StateFunction<TT>>(
        &mut self,
        handle: &KeyedSoundInputHandle<K, TT>,
        f: F,
    ) -> (NumberSourceHandle, ResultFuture<(), ()>) {
        let nsid = self.number_source_idgen.next_id();
        let owner = handle.id();
        let source = Arc::new(KeyedInputNumberSource::new(handle.clone(), f));
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::AddNumberSource {
                id: nsid,
                owner: NumberSourceOwner::SoundInput(owner),
                source,
                result: outbound_result,
            });
        (
            NumberSourceHandle::new(nsid, NumberSourceOwner::SoundInput(owner)),
            result_future,
        )
    }

    // TODO: return the handle only
    pub fn add_number_input(&mut self) -> (NumberInputHandle, ResultFuture<(), ()>) {
        let niid = self.number_input_idgen.next_id();
        let owner = NumberInputOwner::SoundProcessor(self.processor_id);
        let handle = NumberInputHandle::new(niid, owner);
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue.push(SoundEngineMessage::AddNumberInput {
            input: handle.clone(),
            result: outbound_result,
        });
        (handle, result_future)
    }

    // TODO: don't return the result future
    pub fn remove_single_sound_input(
        &mut self,
        handle: SingleSoundInputHandle,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveSoundInput {
                input_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    // TODO: don't return the result future
    pub fn remove_keyed_sound_input<K: Key, TT: SoundState>(
        &mut self,
        handle: KeyedSoundInputHandle<K, TT>,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveSoundInput {
                input_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    // TODO: don't return the result future
    pub fn remove_number_input(&mut self, handle: NumberInputHandle) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberInput {
                input_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    // TODO: don't return the result future
    pub fn remove_processor_number_source(
        &mut self,
        handle: NumberSourceHandle,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberSource {
                source_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    // TODO: don't return the result future
    pub fn remove_single_input_number_source(
        &mut self,
        handle: NumberSourceHandle,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberSource {
                source_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    // TODO: don't return the result future
    pub fn remove_keyed_input_number_source(
        &mut self,
        handle: NumberSourceHandle,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberSource {
                source_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    pub(super) fn add_keyed_input_key(
        &mut self,
        input_id: SoundInputId,
        key: TypeErasedKey,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::AddSoundInputKey {
                input_id,
                key,
                result: outbound_result,
            });
        result_future
    }

    pub(super) fn remove_keyed_input_key(
        &mut self,
        input_id: SoundInputId,
        key_index: usize,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveSoundInputKey {
                input_id,
                key_index,
                result: outbound_result,
            });
        result_future
    }

    pub(super) fn deliver_messages(&mut self, sender: &'a Sender<SoundEngineMessage>) {
        let msgs = std::mem::take(&mut self.message_queue);
        for m in msgs {
            sender.send(m).unwrap();
        }
    }
}
