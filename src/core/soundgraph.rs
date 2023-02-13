use std::{collections::HashSet, sync::Arc};

use super::{
    graphobject::{ObjectId, ObjectInitialization},
    numberinput::NumberInputId,
    numbersource::{
        NumberSourceId, NumberSourceOwner, PureNumberSource, PureNumberSourceHandle,
        PureNumberSourceWithId,
    },
    numbersourcetools::NumberSourceTools,
    soundengine::{SoundEngine, SoundEngineInterface},
    soundgraphdata::{NumberSourceData, SoundProcessorData},
    soundgraphedit::SoundGraphEdit,
    soundgrapherror::{NumberError, SoundGraphError},
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::find_error,
    soundinput::SoundInputId,
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
    },
    soundprocessortools::SoundProcessorTools,
    uniqueid::IdGenerator,
};

struct SoundGraphClosure {
    sound_processors: HashSet<SoundProcessorId>,
    sound_inputs: HashSet<SoundInputId>,
    number_sources: HashSet<NumberSourceId>,
    number_inputs: HashSet<NumberInputId>,
}

impl SoundGraphClosure {
    fn new() -> SoundGraphClosure {
        SoundGraphClosure {
            sound_processors: HashSet::new(),
            sound_inputs: HashSet::new(),
            number_sources: HashSet::new(),
            number_inputs: HashSet::new(),
        }
    }

    fn add_sound_processor(&mut self, id: SoundProcessorId, topology: &SoundGraphTopology) {
        let was_added = self.sound_processors.insert(id);
        if !was_added {
            return;
        }
        let data = topology.sound_processor(id).unwrap();
        for siid in data.sound_inputs() {
            self.add_sound_input(*siid, topology);
        }
        for nsid in data.number_sources() {
            self.add_number_source(*nsid, topology);
        }
        for niid in data.number_inputs() {
            self.add_number_input(*niid);
        }
    }

    fn add_sound_input(&mut self, id: SoundInputId, topology: &SoundGraphTopology) {
        let was_added = self.sound_inputs.insert(id);
        if !was_added {
            return;
        }
        let data = topology.sound_input(id).unwrap();
        for nsid in data.number_sources() {
            self.add_number_source(*nsid, topology);
        }
    }

    fn add_number_source(&mut self, id: NumberSourceId, topology: &SoundGraphTopology) {
        let was_added = self.number_sources.insert(id);
        if !was_added {
            return;
        }
        let data = topology.number_source(id).unwrap();
        for niid in data.inputs() {
            self.add_number_input(*niid);
        }
    }

    fn add_number_input(&mut self, id: NumberInputId) {
        self.number_inputs.insert(id);
    }

    fn includes_sound_connection(&self, id: SoundInputId, topology: &SoundGraphTopology) -> bool {
        if self.sound_inputs.contains(&id) {
            return true;
        }
        let data = topology.sound_input(id).unwrap();
        if let Some(spid) = data.target() {
            if self.sound_processors.contains(&spid) {
                return true;
            }
        }
        false
    }

    fn includes_number_connection(&self, id: NumberInputId, topology: &SoundGraphTopology) -> bool {
        if self.number_inputs.contains(&id) {
            return true;
        }
        let data = topology.number_input(id).unwrap();
        if let Some(spid) = data.target() {
            if self.number_sources.contains(&spid) {
                return true;
            }
        }
        false
    }
}

pub struct SoundGraph {
    local_topology: SoundGraphTopology,

    engine_interface: SoundEngineInterface,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<NumberSourceId>,
    number_input_idgen: IdGenerator<NumberInputId>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        let engine_interface = SoundEngine::spawn();
        SoundGraph {
            engine_interface,

            local_topology: SoundGraphTopology::new(),

            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
        }
    }

    pub fn add_static_sound_processor<T: StaticSoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<StaticSoundProcessorHandle<T>, ()> {
        let id = self.sound_processor_idgen.next_id();
        let mut edit_queue = Vec::new();
        let processor;
        {
            let tools = self.make_tools_for(id, &mut edit_queue);
            let p = T::new(tools, init)?;
            processor = Arc::new(StaticSoundProcessorWithId::new(p, id));
        }
        let processor2 = Arc::clone(&processor);
        let data = SoundProcessorData::new(processor);
        edit_queue.insert(0, SoundGraphEdit::AddSoundProcessor(data));
        self.try_make_edits(edit_queue).unwrap();
        Ok(StaticSoundProcessorHandle::new(processor2))
    }

    pub fn add_dynamic_sound_processor<T: DynamicSoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<DynamicSoundProcessorHandle<T>, ()> {
        let id = self.sound_processor_idgen.next_id();
        let mut edit_queue = Vec::new();
        let processor;
        {
            let tools = self.make_tools_for(id, &mut edit_queue);
            let p = T::new(tools, init)?;
            processor = Arc::new(DynamicSoundProcessorWithId::new(p, id));
        }
        let processor2 = Arc::clone(&processor);
        let data = SoundProcessorData::new(processor);
        edit_queue.insert(0, SoundGraphEdit::AddSoundProcessor(data));
        self.try_make_edits(edit_queue).unwrap();
        Ok(DynamicSoundProcessorHandle::new(processor2))
    }

    pub fn add_pure_number_source<T: PureNumberSource>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<PureNumberSourceHandle<T>, ()> {
        let id = self.number_source_idgen.next_id();
        let owner = NumberSourceOwner::Nothing;
        let mut edit_queue = Vec::new();
        let instance;
        {
            let tools = NumberSourceTools::new(id, &mut self.number_input_idgen, &mut edit_queue);
            let s = T::new(tools, init)?;
            instance = Arc::new(PureNumberSourceWithId::new(s, id));
        }
        let instance2 = Arc::clone(&instance);
        let data = NumberSourceData::new(id, instance, owner);
        edit_queue.insert(0, SoundGraphEdit::AddNumberSource(data));
        self.try_make_edits(edit_queue).unwrap();
        Ok(PureNumberSourceHandle::new(instance2))
    }

    pub fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundGraphError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::ConnectSoundInput(input_id, processor_id));
        self.try_make_edits(edit_queue)
    }

    pub fn disconnect_number_input(&mut self, input_id: NumberInputId) -> Result<(), NumberError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::DisconnectNumberInput(input_id));
        self.try_make_edits(edit_queue)
            .map_err(|e| e.into_number().unwrap())
    }

    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Result<(), NumberError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::ConnectNumberInput(input_id, source_id));
        self.try_make_edits(edit_queue)
            .map_err(|e| e.into_number().unwrap())
    }

    pub fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), SoundGraphError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::DisconnectSoundInput(input_id));
        self.try_make_edits(edit_queue)
    }

    pub fn remove_sound_processor(&mut self, id: SoundProcessorId) -> Result<(), SoundGraphError> {
        self.remove_objects_batch(&[id.into()])
    }

    pub fn remove_pure_number_source(&mut self, id: NumberSourceId) -> Result<(), SoundGraphError> {
        self.remove_objects_batch(&[id.into()])
    }

    pub fn remove_objects_batch(&mut self, objects: &[ObjectId]) -> Result<(), SoundGraphError> {
        let mut closure = SoundGraphClosure::new();
        for oid in objects {
            match oid {
                ObjectId::Sound(spid) => closure.add_sound_processor(*spid, &self.local_topology),
                ObjectId::Number(nsid) => closure.add_number_source(*nsid, &self.local_topology),
            }
        }
        let closure = closure;

        let mut edit_queue = Vec::new();

        // find all number connections involving these objects and disconnect them
        for ni in self.local_topology.number_inputs().values() {
            if ni.target().is_some() {
                if closure.includes_number_connection(ni.id(), &self.local_topology) {
                    edit_queue.push(SoundGraphEdit::DisconnectNumberInput(ni.id()));
                }
            }
        }

        // find all sound connections involving these objects and disconnect them
        for si in self.local_topology.sound_inputs().values() {
            if si.target().is_some() {
                if closure.includes_sound_connection(si.id(), &self.local_topology) {
                    edit_queue.push(SoundGraphEdit::DisconnectSoundInput(si.id()));
                }
            }
        }

        // remove all number inputs
        for niid in &closure.number_inputs {
            let owner = self.local_topology.number_input(*niid).unwrap().owner();
            edit_queue.push(SoundGraphEdit::RemoveNumberInput(*niid, owner));
        }

        // remove all number sources
        for nsid in &closure.number_sources {
            let owner = self.local_topology.number_source(*nsid).unwrap().owner();
            edit_queue.push(SoundGraphEdit::RemoveNumberSource(*nsid, owner));
        }

        // remove all sound inputs
        for siid in &closure.sound_inputs {
            let owner = self.local_topology.sound_input(*siid).unwrap().owner();
            edit_queue.push(SoundGraphEdit::RemoveSoundInput(*siid, owner));
        }

        // remove all sound processors
        for spid in &closure.sound_processors {
            edit_queue.push(SoundGraphEdit::RemoveSoundProcessor(*spid));
        }

        self.try_make_edits(edit_queue)
    }

    pub fn apply_processor_tools<F: Fn(SoundProcessorTools)>(
        &mut self,
        processor_id: SoundProcessorId,
        f: F,
    ) -> Result<(), SoundGraphError> {
        let mut edit_queue = Vec::new();
        {
            let tools = self.make_tools_for(processor_id, &mut edit_queue);
            f(tools);
        }
        self.try_make_edits(edit_queue)
    }

    fn make_tools_for<'a>(
        &'a mut self,
        processor_id: SoundProcessorId,
        edit_queue: &'a mut Vec<SoundGraphEdit>,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools::new(
            processor_id,
            &mut self.sound_input_idgen,
            &mut self.number_input_idgen,
            &mut self.number_source_idgen,
            edit_queue,
        )
    }

    fn try_make_edits(&mut self, edit_queue: Vec<SoundGraphEdit>) -> Result<(), SoundGraphError> {
        let prev_topology = self.local_topology.clone();
        for edit in edit_queue {
            println!("SoundGraph: {}", edit.name());
            debug_assert!(edit.check_preconditions(&self.local_topology));
            self.local_topology.make_edit(edit.clone());
            if let Some(err) = find_error(&self.local_topology) {
                self.local_topology = prev_topology;
                return Err(err);
            }
            self.engine_interface.make_edit(edit);
        }
        Ok(())
    }

    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        &self.local_topology
    }
}
