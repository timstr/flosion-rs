use crate::sound::soundinput::SoundInputId;
use crate::sound::soundprocessor::DynamicSoundProcessor;
use crate::sound::soundprocessor::SoundProcessorId;
use crate::sound::soundprocessor::StaticSoundProcessor;

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use super::connectionerror::ConnectionError;
use super::numberinput::NumberInputId;
use super::numbersource::NumberSourceId;
use super::resultfuture::ResultFuture;
use super::soundengine::SoundEngine;
use super::soundengine::SoundEngineMessage;
use super::soundprocessor::DynamicSoundProcessorData;
use super::soundprocessor::StaticSoundProcessorData;
use super::soundprocessor::WrappedDynamicSoundProcessor;
use super::soundprocessor::WrappedStaticSoundProcessor;
use super::soundprocessortools::SoundProcessorTools;
use super::uniqueid::IdGenerator;

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    wrapper: Arc<WrappedDynamicSoundProcessor<T>>,
    id: SoundProcessorId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub(super) fn wrapper(&self) -> &WrappedDynamicSoundProcessor<T> {
        &*self.wrapper
    }

    pub fn instance(&self) -> &T {
        self.wrapper.instance()
    }
}
pub struct StaticSoundProcessorHandle<T: StaticSoundProcessor> {
    wrapper: Arc<WrappedStaticSoundProcessor<T>>,
    id: SoundProcessorId,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn wrapper(&self) -> &WrappedStaticSoundProcessor<T> {
        &*self.wrapper
    }

    pub fn instance(&self) -> &T {
        self.wrapper.instance()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum AudioError {
    AlreadyStarted,
    AlreadyStopped,
}

pub struct SoundGraph {
    // NOTE: I'd really like to make these two mutually exclusive states into an enum,
    // but rust doesn't have an elegant way to replace a value with something depending
    // on the old value.
    engine_idle: Option<SoundEngine>,
    engine_running: Option<JoinHandle<SoundEngine>>,

    message_sender: Sender<SoundEngineMessage>,
    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        let (eng, tx) = SoundEngine::new();
        SoundGraph {
            engine_idle: Some(eng),
            engine_running: None,
            message_sender: tx,
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
        }
    }

    pub async fn add_dynamic_sound_processor<T: DynamicSoundProcessor + 'static>(
        &mut self,
    ) -> DynamicSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let mut tools = SoundProcessorTools::new(
            id,
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        let data = Arc::new(DynamicSoundProcessorData::<T::StateType>::new(id));
        let processor = Arc::new(T::new(&mut tools, &data));
        let wrapper = Arc::new(WrappedDynamicSoundProcessor::new(
            Arc::clone(&processor),
            data,
        ));
        let wrapper2 = Arc::clone(&wrapper);
        let (result_future, outbound_result) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::AddSoundProcessor {
                processor: wrapper2,
                result: outbound_result,
            })
            .unwrap();
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
        result_future.await.unwrap();
        DynamicSoundProcessorHandle { wrapper, id }
    }

    pub async fn add_static_sound_processor<T: 'static + StaticSoundProcessor>(
        &mut self,
    ) -> StaticSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let mut tools = SoundProcessorTools::new(
            id,
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        let data = Arc::new(StaticSoundProcessorData::<T::StateType>::new(id));
        let processor = Arc::new(T::new(&mut tools, &data));
        let wrapper = Arc::new(WrappedStaticSoundProcessor::new(
            Arc::clone(&processor),
            data,
        ));
        let wrapper2 = Arc::clone(&wrapper);
        let (result_future, outbound_result) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::AddSoundProcessor {
                processor: wrapper2,
                result: outbound_result,
            })
            .unwrap();
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
        result_future.await.unwrap();
        StaticSoundProcessorHandle { wrapper, id }
    }

    pub async fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), ConnectionError> {
        let (result_future, outbound_result) = ResultFuture::<(), ConnectionError>::new();
        self.message_sender
            .send(SoundEngineMessage::ConnectSoundInput {
                input_id,
                processor_id,
                result: outbound_result,
            })
            .unwrap();
        self.flush_idle_messages();
        result_future.await
    }

    pub async fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), ConnectionError> {
        let (rf, result) = ResultFuture::<(), ConnectionError>::new();
        self.message_sender
            .send(SoundEngineMessage::DisconnectSoundInput { input_id, result })
            .unwrap();
        self.flush_idle_messages();
        rf.await
    }

    pub fn start(&mut self) -> ResultFuture<(), ()> {
        debug_assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        if let Some(e) = self.engine_idle.take() {
            let mut e = e;
            self.engine_running = Some(thread::spawn(move || {
                outbound_result.fulfill(Ok(()));
                e.run();
                e
            }));
        } else {
            outbound_result.fulfill(Err(()));
        }
        result_future
    }

    pub fn stop(&mut self) -> ResultFuture<(), ()> {
        debug_assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        if let Some(jh) = self.engine_running.take() {
            self.message_sender
                .send(SoundEngineMessage::Stop {
                    result: outbound_result,
                })
                .unwrap();
            self.engine_idle = Some(jh.join().unwrap());
        } else {
            outbound_result.fulfill(Err(()));
        }
        result_future
    }

    fn flush_idle_messages(&mut self) {
        if let Some(eng) = &mut self.engine_idle {
            eng.flush_messages();
        }
    }
}
