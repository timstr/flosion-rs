use std::sync::{mpsc::Sender, Arc};

use super::{
    key::Key,
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::{
        NumberSourceId, NumberSourceOwner, ProcessorNumberSource, ProcessorNumberSourceHandle,
        StateFunction,
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

    pub fn add_single_input(
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

    pub fn add_keyed_input<K: Key, TT: SoundState>(
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

    pub fn add_number_source<F: StateFunction<T>>(
        &mut self,
        function: F,
    ) -> (ProcessorNumberSourceHandle, ResultFuture<(), ()>) {
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
            ProcessorNumberSourceHandle::new(nsid, self.processor_id),
            result_future,
        )
    }

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

    pub fn remove_single_input(&mut self, handle: SingleSoundInputHandle) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveSoundInput {
                input_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    pub fn remove_keyed_input<K: Key, TT: SoundState>(
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

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberInput {
                input_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    pub fn remove_number_source(
        &mut self,
        handle: ProcessorNumberSourceHandle,
    ) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberSource {
                source_id: handle.id(),
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

// pub struct DynamicSoundProcessorTools<'a, T: SoundState> {
//     base: SoundProcessorTools<'a>,
//     data: Arc<DynamicSoundProcessorData<T>>,
// }

// impl<'a, T: SoundState> DynamicSoundProcessorTools<'a, T> {
//     pub(super) fn new(
//         tools: SoundProcessorTools<'a>,
//         data: Arc<DynamicSoundProcessorData<T>>,
//     ) -> DynamicSoundProcessorTools<'a, T> {
//         DynamicSoundProcessorTools { base: tools, data }
//     }

//     pub fn add_single_input(
//         &mut self,
//         options: InputOptions,
//     ) -> (SingleSoundInputHandle, ResultFuture<(), ()>) {
//         self.base.add_single_input(options)
//     }

//     pub fn add_keyed_input<K: Key, TT: SoundState>(
//         &mut self,
//         options: InputOptions,
//     ) -> (KeyedSoundInputHandle<K, TT>, ResultFuture<(), ()>) {
//         self.base.add_keyed_input(options)
//     }

//     pub fn add_number_source<F: StateFunction<T>>(
//         &mut self,
//         f: F,
//     ) -> (
//         ProcessorNumberSourceHandle<T, F>, // TODO: make this a struct, forget about the generic NumberSourceHandle
//         ResultFuture<(), ()>,
//     ) {
//         self.base.add_number_source()
//     }

//     pub fn add_number_input(&mut self) -> (NumberInputHandle, ResultFuture<(), ()>) {
//         self.base.add_number_input()
//     }

//     pub fn remove_single_input(&mut self, handle: SingleSoundInputHandle) -> ResultFuture<(), ()> {
//         self.base.remove_single_input(handle)
//     }

//     pub fn remove_keyed_input<K: Key, TT: SoundState>(
//         &mut self,
//         handle: KeyedSoundInputHandle<K, TT>,
//     ) -> ResultFuture<(), ()> {
//         self.base.remove_keyed_input(handle)
//     }

//     pub fn remove_number_input(&mut self, handle: NumberInputHandle) -> ResultFuture<(), ()> {
//         self.base.remove_number_input(handle)
//     }

//     pub fn remove_number_source<F: StateFunction<T>>(
//         &mut self,
//         handle: NumberSourceHandle,
//     ) -> ResultFuture<(), ()> {
//         self.base.remove_number_source(handle)
//     }

//     pub(super) fn base(&self) -> &SoundProcessorTools<'a> {
//         &self.base
//     }

//     pub(super) fn base_mut(&mut self) -> &mut SoundProcessorTools<'a> {
//         &mut self.base
//     }
// }

// pub struct StaticSoundProcessorTools<'a, T: SoundState> {
//     base: SoundProcessorTools<'a>,
//     data: Arc<StaticSoundProcessorData<T>>,
// }

// impl<'a, T: SoundState> StaticSoundProcessorTools<'a, T> {
//     pub(super) fn new(
//         tools: SoundProcessorTools<'a>,
//         data: Arc<StaticSoundProcessorData<T>>,
//     ) -> StaticSoundProcessorTools<'a, T> {
//         StaticSoundProcessorTools { base: tools, data }
//     }

//     pub fn add_single_input(
//         &mut self,
//         options: InputOptions,
//     ) -> (SingleSoundInputHandle, ResultFuture<(), ()>) {
//         self.base.add_single_input(options)
//     }

//     pub fn add_keyed_input<K: Key, TT: SoundState>(
//         &mut self,
//         options: InputOptions,
//     ) -> (KeyedSoundInputHandle<K, TT>, ResultFuture<(), ()>) {
//         self.base.add_keyed_input(options)
//     }

//     pub fn add_number_source<F: StateFunction<T>>(
//         &mut self,
//         f: F,
//     ) -> (NumberSourceHandle, ResultFuture<(), ()>) {
//         let ns = StaticProcessorNumberSource::new(Arc::clone(&self.data), f);
//         self.base.add_number_source(Box::new(ns))
//     }

//     pub fn add_number_input(&mut self) -> (NumberInputHandle, ResultFuture<(), ()>) {
//         self.base.add_number_input()
//     }

//     pub fn remove_single_input(&mut self, handle: SingleSoundInputHandle) -> ResultFuture<(), ()> {
//         self.base.remove_single_input(handle)
//     }

//     pub fn remove_keyed_input<K: Key, TT: SoundState>(
//         &mut self,
//         handle: KeyedSoundInputHandle<K, TT>,
//     ) -> ResultFuture<(), ()> {
//         self.base.remove_keyed_input(handle)
//     }

//     pub fn remove_number_input(&mut self, handle: NumberInputHandle) -> ResultFuture<(), ()> {
//         self.base.remove_number_input(handle)
//     }

//     pub fn remove_number_source(&mut self, handle: NumberSourceHandle) -> ResultFuture<(), ()> {
//         self.base.remove_number_source(handle)
//     }

//     pub(super) fn base(&self) -> &SoundProcessorTools<'a> {
//         &self.base
//     }

//     pub(super) fn base_mut(&mut self) -> &mut SoundProcessorTools<'a> {
//         &mut self.base
//     }
// }
