use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

use parking_lot::RwLock;

use super::{
    graphobject::{GraphObject, ObjectId, ObjectInitialization},
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, PureNumberSource, PureNumberSourceHandle},
    soundengine::SoundEngine,
    soundgraphdescription::SoundGraphDescription,
    soundgrapherror::{NumberConnectionError, SoundConnectionError, SoundGraphError},
    soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId,
    soundprocessor::{SoundProcessor, SoundProcessorHandle, SoundProcessorId},
    soundprocessortools::SoundProcessorTools,
};

pub struct SoundGraph {
    // NOTE: I'd really like to make these two mutually exclusive states into an enum,
    // but rust doesn't have an elegant way to replace a value with something depending
    // on the old value.
    engine_idle: Option<SoundEngine>,
    engine_running: Option<JoinHandle<SoundEngine>>,

    keep_running: Arc<AtomicBool>,

    topology: Arc<RwLock<SoundGraphTopology>>,
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
        }
    }

    pub fn add_sound_processor<T: SoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> SoundProcessorHandle<T> {
        self.topology.write().add_sound_processor::<T>(init)
    }

    pub fn add_pure_number_source<T: PureNumberSource>(
        &mut self,
        init: ObjectInitialization,
    ) -> PureNumberSourceHandle<T> {
        self.topology.write().add_pure_number_source::<T>(init)
    }

    pub fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundGraphError> {
        self.topology
            .write()
            .connect_sound_input(input_id, processor_id)
    }

    pub fn sound_input_target(
        &self,
        input_id: SoundInputId,
    ) -> Result<Option<SoundProcessorId>, SoundConnectionError> {
        match self.topology.read().sound_inputs().get(&input_id) {
            Some(i) => Ok(i.target()),
            None => return Err(SoundConnectionError::InputNotFound(input_id)),
        }
    }

    pub fn number_input_target(
        &self,
        input_id: NumberInputId,
    ) -> Result<Option<NumberSourceId>, NumberConnectionError> {
        match self.topology.read().number_inputs().get(&input_id) {
            Some(i) => Ok(i.target()),
            None => return Err(NumberConnectionError::InputNotFound(input_id)),
        }
    }

    pub fn start(&mut self) {
        debug_assert!(self.engine_idle.is_some() != self.engine_running.is_some());
        if let Some(e) = self.engine_idle.take() {
            self.keep_running.store(true, Ordering::SeqCst);
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
        self.topology.read().describe()
    }

    pub fn graph_objects(&self) -> Vec<Box<dyn GraphObject>> {
        let mut ret: Vec<Box<dyn GraphObject>> = Vec::new();
        let topo = self.topology.read();
        for (id, data) in topo.sound_processors() {
            ret.push(data.instance_arc().as_graph_object(*id));
        }
        for (id, data) in topo.number_sources() {
            if let Some(obj) = data.instance_arc().as_graph_object(*id) {
                ret.push(obj);
            }
        }
        ret
    }

    pub fn disconnect_number_input(
        &self,
        input_id: NumberInputId,
    ) -> Result<(), NumberConnectionError> {
        self.topology.write().disconnect_number_input(input_id)
    }

    pub fn connect_number_input(
        &self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Result<(), NumberConnectionError> {
        self.topology
            .write()
            .connect_number_input(input_id, source_id)
    }

    pub fn disconnect_sound_input(&self, input_id: SoundInputId) -> Result<(), SoundGraphError> {
        self.topology.write().disconnect_sound_input(input_id)
    }

    pub fn remove_sound_processor(&self, id: SoundProcessorId) {
        self.topology.write().remove_sound_processor(id)
    }

    pub fn remove_number_source(&self, id: NumberSourceId) {
        self.topology.write().remove_number_source(id)
    }

    pub fn remove_objects<I: Iterator<Item = ObjectId>>(&self, objects: I) {
        self.topology.write().remove_objects(objects);
    }

    pub fn apply_processor_tools<F: Fn(SoundProcessorTools)>(
        &self,
        processor_id: SoundProcessorId,
        f: F,
    ) {
        let mut topo = self.topology.write();
        let tools = SoundProcessorTools::new(processor_id, &mut topo);
        f(tools);
    }

    pub fn topology(&self) -> Arc<RwLock<SoundGraphTopology>> {
        Arc::clone(&self.topology)
    }
}
