use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use parking_lot::RwLock;

use super::{
    graphobject::{GraphObject, ObjectId, WithObjectType},
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, PureNumberSource, PureNumberSourceHandle},
    numbersourcetools::NumberSourceTools,
    serialization::{Archive, Deserializer},
    soundengine::SoundEngine,
    soundgraphdescription::SoundGraphDescription,
    soundgrapherror::{NumberConnectionError, SoundGraphError},
    soundgraphtopology::SoundGraphTopology,
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

pub struct SoundGraph {
    // NOTE: I'd really like to make these two mutually exclusive states into an enum,
    // but rust doesn't have an elegant way to replace a value with something depending
    // on the old value.
    engine_idle: Option<SoundEngine>,
    engine_running: Option<JoinHandle<SoundEngine>>,

    keep_running: Arc<AtomicBool>,

    topology: Arc<RwLock<SoundGraphTopology>>,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,

    graph_objects: Vec<(ObjectId, Arc<dyn GraphObject>)>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        let (engine, keep_running) = SoundEngine::new();
        let topology = engine.topology();
        SoundGraph {
            engine_idle: Some(engine),
            engine_running: None,
            topology,
            keep_running,
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
            graph_objects: Vec::new(),
        }
    }

    pub fn add_dynamic_sound_processor<'a, T: DynamicSoundProcessor + WithObjectType>(
        &mut self,
        deserializer: Option<Deserializer<'a>>,
    ) -> DynamicSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let data = Arc::new(SoundProcessorData::<T::StateType>::new(id, false));
        let mut topo = self.topology.read().clone();
        let mut tools = SoundProcessorTools::new(
            id,
            Arc::clone(&data),
            &mut topo,
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        let processor = if let Some(d) = deserializer {
            Arc::new(T::new_deserialized(&mut tools, d))
        } else {
            Arc::new(T::new_default(&mut tools))
        };
        self.update_topology(topo);
        let wrapper = Arc::new(WrappedDynamicSoundProcessor::new(
            Arc::clone(&processor),
            data,
        ));
        DynamicSoundProcessorHandle { wrapper, id }
    }

    pub fn add_static_sound_processor<'a, T: StaticSoundProcessor + WithObjectType>(
        &mut self,
        deserializer: Option<Deserializer<'a>>,
    ) -> StaticSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let data = Arc::new(SoundProcessorData::<T::StateType>::new(id, true));
        let mut topo = self.topology.read().clone();
        let mut tools = SoundProcessorTools::new(
            id,
            Arc::clone(&data),
            &mut topo,
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        let processor = if let Some(d) = deserializer {
            Arc::new(T::new_deserialized(&mut tools, d))
        } else {
            Arc::new(T::new_default(&mut tools))
        };
        self.update_topology(topo);
        let wrapper = Arc::new(WrappedStaticSoundProcessor::new(
            Arc::clone(&processor),
            data,
        ));
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
        let mut topo = self.topology.read().clone();
        let mut tools = SoundProcessorTools::new(
            wrapper.id(),
            Arc::clone(&wrapper.data()),
            &mut topo,
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        f(wrapper, &mut tools);
        self.update_topology(topo);
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
        let mut topo = self.topology.read().clone();
        let mut tools = SoundProcessorTools::new(
            wrapper.id(),
            Arc::clone(&wrapper.data()),
            &mut topo,
            &mut self.sound_input_idgen,
            &mut self.number_source_idgen,
            &mut self.number_input_idgen,
        );
        f(wrapper, &mut tools);
        self.update_topology(topo);
    }

    pub fn add_number_source<'a, T: PureNumberSource + WithObjectType>(
        &mut self,
        deserializer: Option<Deserializer<'a>>,
    ) -> PureNumberSourceHandle<T> {
        let id = self.number_source_idgen.next_id();
        let mut topo = self.topology.read().clone();
        let mut tools = NumberSourceTools::new(id, &mut topo, &mut self.number_input_idgen);
        let source = if let Some(d) = deserializer {
            Arc::new(T::new_deserialized(&mut tools, d))
        } else {
            Arc::new(T::new_default(&mut tools))
        };
        self.update_topology(topo);
        let handle = PureNumberSourceHandle::new(id, source);
        handle
    }

    pub fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        let mut topo = self.topology.read().clone();
        topo.remove_sound_processor(processor_id);
        self.update_topology(topo);
    }

    pub fn remove_number_source(&mut self, source_id: NumberSourceId) {
        let mut topo = self.topology.read().clone();
        topo.remove_number_source(source_id);
        self.update_topology(topo);
    }

    pub fn graph_objects(&self) -> &[(ObjectId, Arc<dyn GraphObject>)] {
        &self.graph_objects
    }

    pub fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundGraphError> {
        let mut topo = self.topology.read().clone();
        topo.connect_sound_input(input_id, processor_id)?;
        self.update_topology(topo);
        Ok(())
    }

    pub fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), SoundGraphError> {
        let mut topo = self.topology.read().clone();
        topo.disconnect_sound_input(input_id)?;
        self.update_topology(topo);
        Ok(())
    }

    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Result<(), NumberConnectionError> {
        let mut topo = self.topology.read().clone();
        topo.connect_number_input(input_id, source_id)?;
        self.update_topology(topo);
        Ok(())
    }

    pub fn disconnect_number_input(
        &mut self,
        input_id: NumberInputId,
    ) -> Result<(), NumberConnectionError> {
        let mut topo = self.topology.read().clone();
        topo.disconnect_number_input(input_id)?;
        self.update_topology(topo);
        Ok(())
    }

    pub fn start(&mut self) {
        debug_assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        if let Some(e) = self.engine_idle.take() {
            let mut e = e;
            self.engine_running = Some(thread::spawn(move || {
                e.run();
                e
            }));
        }
    }

    pub fn stop(&mut self) {
        debug_assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        if let Some(jh) = self.engine_running.take() {
            self.keep_running.store(false, Ordering::SeqCst);
            self.engine_idle = Some(jh.join().unwrap());
        }
    }

    pub fn is_running(&self) -> bool {
        self.engine_running.is_some()
    }

    pub fn describe(&self) -> SoundGraphDescription {
        let topo = self.topology.read().clone();
        topo.describe()
    }

    // pub fn serialize(&self) -> Archive {
    //     // TODO:
    //     // - serialize object contents and list of pegs
    //     // - serialize connectivity
    //     Archive::serialize_with(|mut s| {
    //         for (object_id, object) in &self.graph_objects {
    //             let mut s2 = s.subarchive();
    //             s2.object(object_id);
    //             s2.
    //         }
    //     })
    // }

    fn update_topology(&mut self, mut new_topology: SoundGraphTopology) {
        {
            let mut topo = self.topology.write();
            // Swap topologies (to avoid waiting for destruction)
            std::mem::swap(&mut *topo, &mut new_topology);
        }
        // contents of old topology are now destroyed after
        // lock is released
    }
}
