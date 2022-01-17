use std::sync::{mpsc::Sender, Arc};

use super::{
    key::Key,
    numberinput::NumberInputId,
    numbersource::{
        DynamicProcessorNumberSource, NumberSource, NumberSourceHandle, NumberSourceId,
        StateFunction,
    },
    resultfuture::ResultFuture,
    soundengine::SoundEngineMessage,
    soundinput::{
        InputOptions, KeyedSoundInput, KeyedSoundInputHandle, SingleSoundInput,
        SingleSoundInputHandle, SoundInputId,
    },
    soundprocessor::{DynamicSoundProcessor, DynamicSoundProcessorData, SoundProcessorId},
    soundstate::SoundState,
    uniqueid::IdGenerator,
};

pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,
    message_queue: Vec<SoundEngineMessage>,
    sound_input_idgen: &'a mut IdGenerator<SoundInputId>,
    number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
}

impl<'a> SoundProcessorTools<'a> {
    pub(super) fn new(
        id: SoundProcessorId,
        input_idgen: &'a mut IdGenerator<SoundInputId>,
        number_source_idgen: &'a mut IdGenerator<NumberSourceId>,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
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

    pub fn add_keyed_input<K: Key + 'static, T: SoundState + 'static>(
        &mut self,
        options: InputOptions,
    ) -> (KeyedSoundInputHandle<K, T>, ResultFuture<(), ()>) {
        let input_id = self.sound_input_idgen.next_id();
        let (input, handle) = KeyedSoundInput::<K, T>::new(input_id, options);
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue.push(SoundEngineMessage::AddSoundInput {
            input,
            owner: self.processor_id,
            result: outbound_result,
        });
        (handle, result_future)
    }

    fn add_number_source(
        &mut self,
        instance: Box<dyn NumberSource>,
    ) -> (NumberSourceHandle, ResultFuture<(), ()>) {
        let nsid = self.number_source_idgen.next_id();
        let handle = NumberSourceHandle::new(nsid, instance);
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::AddNumberSource {
                source: handle.clone(),
                result: outbound_result,
            });
        (handle, result_future)
    }

    pub(super) fn deliver_messages(&mut self, sender: &'a Sender<SoundEngineMessage>) {
        let msgs = std::mem::take(&mut self.message_queue);
        for m in msgs {
            sender.send(m).unwrap();
        }
    }
}

pub struct DynamicSoundProcessorTools<'a, T: DynamicSoundProcessor> {
    base: SoundProcessorTools<'a>,
    data: Arc<DynamicSoundProcessorData<T::StateType>>,
}

impl<'a, T: DynamicSoundProcessor> DynamicSoundProcessorTools<'a, T> {
    pub(super) fn new(
        tools: SoundProcessorTools<'a>,
        data: Arc<DynamicSoundProcessorData<T::StateType>>,
    ) -> DynamicSoundProcessorTools<'a, T> {
        DynamicSoundProcessorTools { base: tools, data }
    }

    pub fn add_single_input(
        &mut self,
        options: InputOptions,
    ) -> (SingleSoundInputHandle, ResultFuture<(), ()>) {
        self.base.add_single_input(options)
    }

    pub fn add_keyed_input<K: Key + 'static, TT: SoundState + 'static>(
        &mut self,
        options: InputOptions,
    ) -> (KeyedSoundInputHandle<K, TT>, ResultFuture<(), ()>) {
        self.base.add_keyed_input(options)
    }

    pub fn add_proccessor_number_source<F: 'static + StateFunction<T::StateType>>(
        &mut self,
        f: F,
    ) -> (NumberSourceHandle, ResultFuture<(), ()>) {
        let ns = DynamicProcessorNumberSource::new(Arc::clone(&self.data), f);
        self.base.add_number_source(Box::new(ns))
    }
}
