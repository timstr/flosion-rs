use std::{
    collections::HashSet,
    sync::{
        mpsc::{channel, Sender},
        Arc,
    },
    thread::JoinHandle,
};

use thread_priority::{set_current_thread_priority, ThreadPriority};

use crate::core::{
    engine::soundengine::{create_sound_engine, StopButton},
    graph::{graph::Graph, graphobject::ObjectInitialization},
    number::numbergraph::NumberGraph,
    uniqueid::IdGenerator,
};

use super::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgraphdata::SoundProcessorData,
    soundgraphedit::SoundGraphEdit,
    soundgrapherror::SoundError,
    soundgraphid::SoundObjectId,
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::find_error,
    soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId,
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
    },
    soundprocessortools::SoundProcessorTools,
};

// TODO: only sound number sources and processor number inputs
struct SoundGraphClosure {
    sound_processors: HashSet<SoundProcessorId>,
    sound_inputs: HashSet<SoundInputId>,
    number_sources: HashSet<SoundNumberSourceId>,
    number_inputs: HashSet<SoundNumberInputId>,
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
            self.add_number_source(*nsid);
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
            self.add_number_source(*nsid);
        }
    }

    fn add_number_source(&mut self, id: SoundNumberSourceId) {
        self.number_sources.insert(id);
    }

    fn add_number_input(&mut self, id: SoundNumberInputId) {
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

    fn includes_number_connection(
        &self,
        id: SoundNumberInputId,
        topology: &SoundGraphTopology,
    ) -> bool {
        if self.number_inputs.contains(&id) {
            return true;
        }
        let data = topology.number_input(id).unwrap();
        for spid in data.targets() {
            if self.number_sources.contains(spid) {
                return true;
            }
        }
        false
    }
}

pub struct SoundGraph {
    local_topology: SoundGraphTopology,

    engine_interface_thread: Option<JoinHandle<()>>,
    stop_button: StopButton,
    topology_sender: Sender<SoundGraphTopology>,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<SoundNumberSourceId>,
    number_input_idgen: IdGenerator<SoundNumberInputId>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        let (sender, receiver) = channel();
        let stop_button = StopButton::new();
        let stop_button_also = stop_button.clone();
        let engine_interface_thread = std::thread::spawn(move || {
            let inkwell_context = inkwell::context::Context::create();
            std::thread::scope(|scope| {
                let (mut engine_interface, engine, garbage_disposer) =
                    create_sound_engine(&inkwell_context, &stop_button_also);

                let engine_thread_handle = scope.spawn(move || {
                    set_current_thread_priority(ThreadPriority::Max).unwrap();
                    engine.run();
                });

                // NOTE: both the engine interface and the garbage disposer
                // deal with LLVM resources and so need to stay on the same
                // thread as the inkwell_context above
                loop {
                    while let Ok(topo) = receiver.try_recv() {
                        if engine_interface.update(topo).is_err() {
                            break;
                        }
                    }
                    garbage_disposer.clear();

                    if engine_thread_handle.is_finished() {
                        break;
                    }

                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            });
        });

        SoundGraph {
            engine_interface_thread: Some(engine_interface_thread),
            stop_button,
            topology_sender: sender,

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
        edit_queue.insert(0, SoundGraphEdit::Sound(SoundEdit::AddSoundProcessor(data)));
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
        edit_queue.insert(0, SoundGraphEdit::Sound(SoundEdit::AddSoundProcessor(data)));
        self.try_make_edits(edit_queue).unwrap();
        Ok(DynamicSoundProcessorHandle::new(processor2))
    }

    pub fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::Sound(SoundEdit::ConnectSoundInput(
            input_id,
            processor_id,
        )));
        self.try_make_edits(edit_queue)
    }

    pub fn disconnect_sound_input(&mut self, input_id: SoundInputId) -> Result<(), SoundError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::Sound(SoundEdit::DisconnectSoundInput(
            input_id,
        )));
        self.try_make_edits(edit_queue)
    }

    pub fn connect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) -> Result<(), SoundError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::Number(SoundNumberEdit::ConnectNumberInput(
            input_id, source_id,
        )));
        self.try_make_edits(edit_queue)
    }

    pub fn disconnect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) -> Result<(), SoundError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::Number(
            SoundNumberEdit::DisconnectNumberInput(input_id, source_id),
        ));
        self.try_make_edits(edit_queue)
    }

    pub fn remove_sound_processor(&mut self, id: SoundProcessorId) -> Result<(), SoundError> {
        self.remove_objects_batch(&[id.into()])
    }

    pub fn remove_objects_batch(&mut self, objects: &[SoundObjectId]) -> Result<(), SoundError> {
        let mut closure = SoundGraphClosure::new();
        for oid in objects {
            match oid {
                SoundObjectId::Sound(spid) => {
                    closure.add_sound_processor(*spid, &self.local_topology)
                }
            }
        }
        let closure = closure;

        let mut edit_queue = Vec::new();

        // find all number connections involving these objects and disconnect them
        for ni in self.local_topology.number_inputs().values() {
            for target in ni.targets() {
                if closure.includes_number_connection(ni.id(), &self.local_topology) {
                    edit_queue.push(SoundGraphEdit::Number(
                        SoundNumberEdit::DisconnectNumberInput(ni.id(), *target),
                    ));
                }
            }
        }

        // find all sound connections involving these objects and disconnect them
        for si in self.local_topology.sound_inputs().values() {
            if si.target().is_some() {
                if closure.includes_sound_connection(si.id(), &self.local_topology) {
                    edit_queue.push(SoundGraphEdit::Sound(SoundEdit::DisconnectSoundInput(
                        si.id(),
                    )));
                }
            }
        }

        // remove all number inputs
        for niid in &closure.number_inputs {
            let owner = self.local_topology.number_input(*niid).unwrap().owner();
            edit_queue.push(SoundGraphEdit::Number(SoundNumberEdit::RemoveNumberInput(
                *niid, owner,
            )));
        }

        // remove all number sources
        for nsid in &closure.number_sources {
            let owner = self.local_topology.number_source(*nsid).unwrap().owner();
            edit_queue.push(SoundGraphEdit::Number(SoundNumberEdit::RemoveNumberSource(
                *nsid, owner,
            )));
        }

        // remove all sound inputs
        for siid in &closure.sound_inputs {
            let owner = self.local_topology.sound_input(*siid).unwrap().owner();
            edit_queue.push(SoundGraphEdit::Sound(SoundEdit::RemoveSoundInput(
                *siid, owner,
            )));
        }

        // remove all sound processors
        for spid in &closure.sound_processors {
            edit_queue.push(SoundGraphEdit::Sound(SoundEdit::RemoveSoundProcessor(
                *spid,
            )));
        }

        self.try_make_edits(edit_queue)
    }

    pub fn apply_processor_tools<F: FnOnce(SoundProcessorTools)>(
        &mut self,
        processor_id: SoundProcessorId,
        f: F,
    ) -> Result<(), SoundError> {
        let mut edit_queue = Vec::new();
        {
            let tools = self.make_tools_for(processor_id, &mut edit_queue);
            f(tools);
        }
        self.try_make_edits(edit_queue)
    }

    pub fn edit_number_input<F: FnOnce(&mut NumberGraph)>(
        &mut self,
        input_id: SoundNumberInputId,
        f: F,
    ) -> Result<(), SoundError> {
        self.try_make_change(|topo| {
            let number_input = topo
                .number_input_mut(input_id)
                .ok_or_else(|| SoundError::NumberInputNotFound(input_id))?;

            number_input.edit_number_graph(f);

            Ok(())
        })
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

    fn try_make_edits_locally(
        topo: &mut SoundGraphTopology,
        edit_queue: Vec<SoundGraphEdit>,
    ) -> Result<(), SoundError> {
        for edit in edit_queue {
            if let Some(err) = edit.check_preconditions(&topo) {
                return Err(err);
            }
            topo.make_sound_graph_edit(edit);
            if let Some(err) = find_error(&topo) {
                return Err(err);
            }
        }
        Ok(())
    }

    fn try_make_edits(&mut self, edit_queue: Vec<SoundGraphEdit>) -> Result<(), SoundError> {
        self.try_make_change(|topo| Self::try_make_edits_locally(topo, edit_queue))
    }

    fn try_make_change<F: FnOnce(&mut SoundGraphTopology) -> Result<(), SoundError>>(
        &mut self,
        f: F,
    ) -> Result<(), SoundError> {
        // TODO: add a separate flush() or commit() method that
        // sends updates to the sound engine on an opt-in basis,
        // e.g. at most once per UI update, rather than sending
        // an update for every smallest change
        debug_assert!(find_error(&self.local_topology).is_none());
        let prev_topology = self.local_topology.clone();
        let res = f(&mut self.local_topology);
        if res.is_err() {
            self.local_topology = prev_topology;
        } else {
            debug_assert!(find_error(&self.local_topology).is_none());
            self.topology_sender
                .send(self.local_topology.clone())
                .unwrap();
        }
        res
    }

    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        &self.local_topology
    }
}

impl Drop for SoundGraph {
    fn drop(&mut self) {
        self.stop_button.stop();
        let engine_interface_thread = self.engine_interface_thread.take().unwrap();
        engine_interface_thread.join().unwrap();
    }
}

impl Graph for SoundGraph {
    type ObjectId = SoundObjectId;
}
