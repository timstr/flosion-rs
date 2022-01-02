use crate::sound::soundinput::SoundInputId;
use crate::sound::soundprocessor::DynamicSoundProcessor;
use crate::sound::soundprocessor::SoundProcessorId;
use crate::sound::soundprocessor::StaticSoundProcessor;

use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use super::connectionerror::ConnectionError;
use super::resultfuture::ResultFuture;
use super::soundengine::SoundEngine;
use super::soundengine::SoundEngineMessage;
use super::soundprocessor::WrappedDynamicSoundProcessor;
use super::soundprocessor::WrappedStaticSoundProcessor;
use super::soundprocessortools::SoundProcessorTools;
use super::uniqueid::IdGenerator;

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    instance: Arc<T>,
    id: SoundProcessorId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn instance(&self) -> &T {
        &*self.instance
    }
}
pub struct StaticSoundProcessorHandle<T: StaticSoundProcessor> {
    instance: Arc<T>,
    id: SoundProcessorId,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn instance(&self) -> &T {
        &*self.instance
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
        }
    }

    pub async fn add_dynamic_sound_processor<T: DynamicSoundProcessor + 'static>(
        &mut self,
    ) -> DynamicSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let mut tools = SoundProcessorTools::new(id, &mut self.sound_input_idgen);
        let processor = Arc::new(T::new(&mut tools));
        let wrapper = WrappedDynamicSoundProcessor::new(Arc::clone(&processor), id);
        let wrapper = Box::new(wrapper);
        let (rf, obr) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::AddSoundProcessor(id, wrapper, obr))
            .unwrap();
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
        rf.await.unwrap();
        DynamicSoundProcessorHandle {
            instance: processor,
            id,
        }
    }

    pub async fn add_static_sound_processor<T: 'static + StaticSoundProcessor>(
        &mut self,
    ) -> StaticSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let mut tools = SoundProcessorTools::new(id, &mut self.sound_input_idgen);
        let processor = Arc::new(T::new(&mut tools));
        let wrapper = WrappedStaticSoundProcessor::new(Arc::clone(&processor), id);
        let wrapper = Box::new(wrapper);
        let (rf, obr) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::AddSoundProcessor(id, wrapper, obr))
            .unwrap();
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
        rf.await.unwrap();
        StaticSoundProcessorHandle {
            instance: processor,
            id,
        }
    }

    pub async fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), ConnectionError> {
        let (rf, obr) = ResultFuture::<(), ConnectionError>::new();
        self.message_sender
            .send(SoundEngineMessage::ConnectInput(
                input_id,
                processor_id,
                obr,
            ))
            .unwrap();
        self.flush_idle_messages();
        rf.await
    }

    pub async fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), ConnectionError> {
        let (rf, obr) = ResultFuture::<(), ConnectionError>::new();
        self.message_sender
            .send(SoundEngineMessage::DisconnectInput(input_id, obr))
            .unwrap();
        self.flush_idle_messages();
        rf.await
    }

    pub fn start(&mut self) -> Result<(), AudioError> {
        assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        if let Some(e) = self.engine_idle.take() {
            let mut e = e;
            self.engine_running = Some(thread::spawn(move || {
                e.run();
                e
            }));
            Ok(())
        } else {
            Err(AudioError::AlreadyStarted)
        }
    }

    pub fn stop(&mut self) -> Result<(), AudioError> {
        assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        if let Some(jh) = self.engine_running.take() {
            self.message_sender.send(SoundEngineMessage::Stop).unwrap();
            self.engine_idle = Some(jh.join().unwrap());
            Ok(())
        } else {
            Err(AudioError::AlreadyStarted)
        }
    }

    fn flush_idle_messages(&mut self) {
        if let Some(eng) = &mut self.engine_idle {
            eng.flush_messages();
        }
    }
}
