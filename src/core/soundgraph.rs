use std::{
    sync::{mpsc::Sender, Arc},
    thread::{self, JoinHandle},
};

use parking_lot::RwLock;

use super::{
    graphobject::{GraphObject, ObjectId, WithObjectType},
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, NumberSourceOwner, PureNumberSource, PureNumberSourceHandle},
    numbersourcetools::NumberSourceTools,
    resultfuture::ResultFuture,
    soundengine::{SoundEngine, SoundEngineMessage},
    soundgraphdescription::SoundGraphDescription,
    soundgrapherror::{NumberConnectionError, SoundGraphError},
    soundinput::SoundInputId,
    soundprocessor::{
        DynamicSoundProcessor, SoundProcessorData, SoundProcessorId, StaticSoundProcessor,
        WrappedDynamicSoundProcessor, WrappedStaticSoundProcessor,
    },
    soundprocessortools::SoundProcessorTools,
    uniqueid::IdGenerator,
};

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    wrapper: Arc<WrappedDynamicSoundProcessor<T>>,
    id: SoundProcessorId,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn wrapper(&self) -> &WrappedDynamicSoundProcessor<T> {
        &*self.wrapper
    }

    pub fn instance(&self) -> &T {
        self.wrapper.instance()
    }

    pub fn num_states(&self) -> usize {
        self.wrapper.num_states()
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
    description: Arc<RwLock<SoundGraphDescription>>,
    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,

    graph_objects: Vec<(ObjectId, Arc<dyn GraphObject>)>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        let (eng, tx) = SoundEngine::new();
        let description = eng.latest_description();
        SoundGraph {
            engine_idle: Some(eng),
            engine_running: None,
            message_sender: tx,
            description,
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
            graph_objects: Vec::new(),
        }
    }

    pub async fn add_dynamic_sound_processor<T: DynamicSoundProcessor + WithObjectType>(
        &mut self,
    ) -> DynamicSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let data = Arc::new(SoundProcessorData::<T::StateType>::new(id, false));
        let mut tools = SoundProcessorTools::new(
            id,
            Arc::clone(&data),
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        let processor = Arc::new(T::new(&mut tools));
        let wrapper = Arc::new(WrappedDynamicSoundProcessor::new(
            Arc::clone(&processor),
            data,
        ));
        let wrapper2 = Arc::clone(&wrapper);
        let wrapper3 = Arc::clone(&wrapper);
        self.graph_objects.push((ObjectId::Sound(id), wrapper3));
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

    pub async fn add_static_sound_processor<T: StaticSoundProcessor + WithObjectType>(
        &mut self,
    ) -> StaticSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let data = Arc::new(SoundProcessorData::<T::StateType>::new(id, true));
        let mut tools = SoundProcessorTools::new(
            id,
            Arc::clone(&data),
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        let processor = Arc::new(T::new(&mut tools));
        let wrapper = Arc::new(WrappedStaticSoundProcessor::new(
            Arc::clone(&processor),
            data,
        ));
        let wrapper2 = Arc::clone(&wrapper);
        let wrapper3 = Arc::clone(&wrapper);
        self.graph_objects.push((ObjectId::Sound(id), wrapper3));
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

    pub fn apply_dynamic_processor_tools<
        'a,
        T: DynamicSoundProcessor,
        F: Fn(&WrappedDynamicSoundProcessor<T>, &mut SoundProcessorTools<'_, T::StateType>),
    >(
        &'a mut self,
        wrapper: &WrappedDynamicSoundProcessor<T>,
        f: F,
    ) {
        let mut tools = SoundProcessorTools::new(
            wrapper.id(),
            Arc::clone(&wrapper.data()),
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        f(wrapper, &mut tools);
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
    }

    pub fn apply_static_processor_tools<
        'a,
        T: StaticSoundProcessor,
        F: Fn(&WrappedStaticSoundProcessor<T>, &mut SoundProcessorTools<T::StateType>),
    >(
        &'a mut self,
        wrapper: &WrappedStaticSoundProcessor<T>,
        f: F,
    ) {
        let mut tools = SoundProcessorTools::new(
            wrapper.id(),
            Arc::clone(&wrapper.data()),
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        f(wrapper, &mut tools);
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
    }

    pub async fn add_number_source<T: PureNumberSource + WithObjectType>(
        &mut self,
    ) -> PureNumberSourceHandle<T> {
        let id = self.number_source_idgen.next_id();
        let mut tools = NumberSourceTools::new(id, &mut self.number_input_idgen);
        let source = Arc::new(T::new(&mut tools));
        let source2 = Arc::clone(&source);
        let source3 = Arc::clone(&source);
        self.graph_objects.push((ObjectId::Number(id), source3));
        let handle = PureNumberSourceHandle::new(id, source);
        let (result_future, outbound_result) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::AddNumberSource {
                id,
                result: outbound_result,
                owner: NumberSourceOwner::Nothing,
                source: source2,
            })
            .unwrap();
        tools.deliver_messages(&mut self.message_sender);
        self.flush_idle_messages();
        result_future.await.unwrap();
        handle
    }

    pub async fn remove_dynamic_sound_processor<T: DynamicSoundProcessor>(
        &mut self,
        handle: DynamicSoundProcessorHandle<T>,
    ) {
        let (result_future, outbound_result) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::RemoveSoundProcessor {
                processor_id: handle.id(),
                result: outbound_result,
            })
            .unwrap();
        self.flush_idle_messages();
        result_future.await.unwrap();
    }

    pub async fn remove_static_number_processor<T: StaticSoundProcessor>(
        &mut self,
        handle: StaticSoundProcessorHandle<T>,
    ) {
        let (result_future, outbound_result) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::RemoveSoundProcessor {
                processor_id: handle.id(),
                result: outbound_result,
            })
            .unwrap();
        self.flush_idle_messages();
        result_future.await.unwrap();
    }

    pub async fn remove_number_source<T: PureNumberSource>(
        &mut self,
        handle: PureNumberSourceHandle<T>,
    ) {
        let (result_future, outbound_result) = ResultFuture::new();
        self.message_sender
            .send(SoundEngineMessage::RemoveNumberSource {
                source_id: handle.id(),
                result: outbound_result,
            })
            .unwrap();
        self.flush_idle_messages();
        result_future.await.unwrap();
    }

    pub fn graph_objects(&self) -> &[(ObjectId, Arc<dyn GraphObject>)] {
        &self.graph_objects
    }

    pub async fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundGraphError> {
        let (result_future, outbound_result) = ResultFuture::<(), SoundGraphError>::new();
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
    ) -> Result<(), SoundGraphError> {
        let (rf, result) = ResultFuture::<(), SoundGraphError>::new();
        self.message_sender
            .send(SoundEngineMessage::DisconnectSoundInput { input_id, result })
            .unwrap();
        self.flush_idle_messages();
        rf.await
    }

    pub async fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Result<(), NumberConnectionError> {
        let (rf, result) = ResultFuture::<(), NumberConnectionError>::new();
        self.message_sender
            .send(SoundEngineMessage::ConnectNumberInput {
                input_id,
                target_id: source_id,
                result,
            })
            .unwrap();
        self.flush_idle_messages();
        rf.await
    }

    pub async fn disconnect_number_input(
        &mut self,
        input_id: NumberInputId,
    ) -> Result<(), NumberConnectionError> {
        let (rf, result) = ResultFuture::<(), NumberConnectionError>::new();
        self.message_sender
            .send(SoundEngineMessage::DisconnectNumberInput { input_id, result })
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

    pub fn is_running(&self) -> bool {
        self.engine_running.is_some()
    }

    pub fn describe(&self) -> SoundGraphDescription {
        self.description.read().clone()
    }

    fn flush_idle_messages(&mut self) {
        if let Some(eng) = &mut self.engine_idle {
            eng.flush_messages();
        }
    }
}
