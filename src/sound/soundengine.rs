use std::{
    collections::HashMap,
    sync::mpsc::TryRecvError,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

use thread_priority::{set_current_thread_priority, ThreadPriority};

use super::{
    connectionerror::{ConnectionError, NumberConnectionError},
    context::{Context, SoundProcessorFrame, SoundStackFrame},
    gridspan::GridSpan,
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceHandle, NumberSourceId},
    resultfuture::OutboundResult,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundgraphdescription::{
        SoundGraphDescription, SoundInputDescription, SoundProcessorDescription,
    },
    soundinput::{SoundInputId, SoundInputWrapper},
    soundprocessor::{SoundProcessorId, SoundProcessorWrapper},
};

#[derive(Copy, Clone)]
pub enum StateOperation {
    Insert,
    Erase,
}

pub struct EngineSoundInputData {
    input: Arc<dyn SoundInputWrapper>,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
}

impl EngineSoundInputData {
    pub fn new(input: Arc<dyn SoundInputWrapper>, owner: SoundProcessorId) -> EngineSoundInputData {
        EngineSoundInputData {
            input,
            target: None,
            owner,
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.input.id()
    }

    pub fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub fn input(&self) -> &dyn SoundInputWrapper {
        &*self.input
    }

    pub fn owner(&self) -> SoundProcessorId {
        self.owner
    }
}

pub struct EngineSoundProcessorData {
    id: SoundProcessorId,
    wrapper: Arc<dyn SoundProcessorWrapper>,
    inputs: Vec<SoundInputId>,
}

impl EngineSoundProcessorData {
    pub fn new(
        wrapper: Arc<dyn SoundProcessorWrapper>,
        id: SoundProcessorId,
    ) -> EngineSoundProcessorData {
        EngineSoundProcessorData {
            id,
            wrapper,
            inputs: Vec::new(),
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn inputs(&self) -> &Vec<SoundInputId> {
        &self.inputs
    }

    pub fn inputs_mut(&mut self) -> &mut Vec<SoundInputId> {
        &mut self.inputs
    }

    pub fn sound_processor(&self) -> &dyn SoundProcessorWrapper {
        &*self.wrapper
    }
}

pub struct EngineNumberInputData {
    id: NumberInputId,
    target: Option<NumberSourceId>,
    owner: NumberInputOwner,
}

pub struct EngineNumberSourceData {
    id: NumberSourceId,
    wrapper: Arc<dyn NumberSource>,
    inputs: Vec<NumberInputId>,
}

pub enum SoundEngineMessage {
    AddSoundProcessor(Arc<dyn SoundProcessorWrapper>, OutboundResult<(), ()>),
    RemoveSoundProcessor(SoundProcessorId, OutboundResult<(), ()>),
    AddSoundInput(Arc<dyn SoundInputWrapper>, SoundProcessorId),
    RemoveSoundInput(SoundInputId),

    ConnectInput(
        SoundInputId,
        SoundProcessorId,
        OutboundResult<(), ConnectionError>,
    ),
    DisconnectInput(SoundInputId, OutboundResult<(), ConnectionError>),

    AddNumberSource(NumberSourceHandle),
    RemoveNumberSource(NumberSourceId),
    AddNumberInput(NumberInputId, NumberInputOwner),
    RemoveNumberInput(NumberInputId),

    ConnectNumberInput(
        NumberInputId,
        NumberSourceId,
        OutboundResult<(), NumberConnectionError>,
    ),
    DisconnectNumberInput(NumberInputId, OutboundResult<(), NumberConnectionError>),

    Stop,
}

pub struct SoundEngine {
    // TODO: divide this into separate hashmaps for:
    // - sound processors (with list of sound input ids and number source ids that it owns)
    // - sound inputs (with SoundProcessorId of owner)
    // - number sources (with either SoundProcessorId/SoundInputId of owner or number input ids that it owns, as in the old stateful/stateless number source distinction)
    // - number inputs (with NumberSourceId/SoundProcessorId of owner)z
    sound_processors: HashMap<SoundProcessorId, EngineSoundProcessorData>,
    sound_inputs: HashMap<SoundInputId, EngineSoundInputData>,
    number_sources: HashMap<NumberSourceId, EngineNumberSourceData>,
    number_inputs: HashMap<NumberInputId, EngineNumberInputData>,
    receiver: Receiver<SoundEngineMessage>,
}

pub enum PlaybackStatus {
    Continue,
    Stop,
}

impl SoundEngine {
    pub fn new() -> (SoundEngine, Sender<SoundEngineMessage>) {
        let (tx, rx) = channel();
        (
            SoundEngine {
                sound_processors: HashMap::new(),
                sound_inputs: HashMap::new(),
                number_sources: HashMap::new(),
                number_inputs: HashMap::new(),
                receiver: rx,
            },
            tx,
        )
    }

    fn add_sound_processor(&mut self, wrapper: Arc<dyn SoundProcessorWrapper>) {
        let processor_id = wrapper.id();
        debug_assert!(
            self.sound_processors.get(&processor_id).is_none(),
            "The processor id should not already be in use"
        );
        let data = EngineSoundProcessorData::new(wrapper, processor_id);
        self.sound_processors.insert(processor_id, data);
    }

    fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // TODO
        panic!()
    }

    fn add_sound_input(
        &mut self,
        processor_id: SoundProcessorId,
        input: Arc<dyn SoundInputWrapper>,
    ) {
        debug_assert!(
            self.sound_processors
                .iter()
                .find_map(|(_, pd)| pd.inputs.iter().find(|i| **i == input.id()))
                .is_none(),
            "The input id should not already be associated with any sound processors"
        );
        debug_assert!(
            self.sound_inputs.get(&input.id()).is_none(),
            "The input id should not already be in use by a sound input"
        );
        let proc_data = self.sound_processors.get_mut(&processor_id).unwrap();
        proc_data.inputs_mut().push(input.id());
        let input_data = EngineSoundInputData::new(input, processor_id);
        self.sound_inputs.insert(input_data.id(), input_data);
    }

    fn remove_sound_input(&mut self, input_id: SoundInputId) {
        // TODO
        panic!()
    }

    fn modify_states_recursively(
        &mut self,
        proc_id: SoundProcessorId,
        dst_states: GridSpan,
        dst_iid: SoundInputId,
        operation: StateOperation,
    ) {
        let mut outbound_connections: Vec<(SoundProcessorId, GridSpan, SoundInputId)> = Vec::new();

        let proc_data = self.sound_processors.get_mut(&proc_id).unwrap();
        let proc = &mut proc_data.sound_processor();
        let gs = match operation {
            StateOperation::Insert => proc.insert_dst_states(dst_iid, dst_states),
            StateOperation::Erase => proc.erase_dst_states(dst_iid, dst_states),
        };
        if proc.is_static() {
            return;
        }
        for i in proc_data.inputs() {
            let input_data = self.sound_inputs.get_mut(&i).unwrap();
            let gsi = match operation {
                StateOperation::Insert => input_data.input().insert_states(gs),
                StateOperation::Erase => input_data.input().erase_states(gs),
            };
            if let Some(pid) = input_data.target {
                outbound_connections.push((pid, gsi, input_data.id()));
            };
        }

        for (pid, gsi, iid) in outbound_connections {
            self.modify_states_recursively(pid, gsi, iid, operation);
        }
    }

    fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), ConnectionError> {
        let mut desc = self.describe();
        assert!(desc.find_error().is_none());

        if let Some(err) = desc.add_connection(input_id, processor_id) {
            return Err(err);
        }

        if let Some(err) = desc.find_error() {
            return Err(err);
        }

        let input_data = self.sound_inputs.get_mut(&input_id);
        if input_data.is_none() {
            return Err(ConnectionError::InputNotFound);
        }
        let input_data = input_data.unwrap();
        if let Some(pid) = input_data.target {
            if pid == processor_id {
                return Err(ConnectionError::NoChange);
            }
            return Err(ConnectionError::InputOccupied);
        }
        input_data.target = Some(processor_id);

        {
            let proc_data = self.sound_processors.get_mut(&processor_id);
            if proc_data.is_none() {
                return Err(ConnectionError::ProcessorNotFound);
            }
            let proc_data = proc_data.unwrap();
            proc_data.sound_processor().add_dst(input_id);
        }

        let input_proc_states = self
            .sound_processors
            .get(&input_data.owner())
            .unwrap()
            .sound_processor()
            .num_states();

        self.modify_states_recursively(
            processor_id,
            GridSpan::new_contiguous(0, input_proc_states),
            input_id,
            StateOperation::Insert,
        );

        Ok(())
    }

    fn disconnect_sound_input(&mut self, input_id: SoundInputId) -> Result<(), ConnectionError> {
        let mut desc = self.describe();
        assert!(desc.find_error().is_none());

        if let Some(err) = desc.remove_connection(input_id) {
            return Err(err);
        }

        if let Some(err) = desc.find_error() {
            return Err(err);
        }

        let input_data = self.sound_inputs.get_mut(&input_id);
        if input_data.is_none() {
            return Err(ConnectionError::InputNotFound);
        }
        let input_data = input_data.unwrap();
        let processor_id = match input_data.target {
            Some(pid) => pid,
            None => return Err(ConnectionError::NoChange),
        };

        input_data.target = None;

        let input_proc_states = self
            .sound_processors
            .get(&input_data.owner())
            .unwrap()
            .sound_processor()
            .num_states();

        self.modify_states_recursively(
            processor_id,
            GridSpan::new_contiguous(0, input_proc_states),
            input_id,
            StateOperation::Erase,
        );

        {
            let proc_data = self.sound_processors.get_mut(&processor_id);
            if proc_data.is_none() {
                return Err(ConnectionError::ProcessorNotFound);
            }
            let proc_data = proc_data.unwrap();
            proc_data.sound_processor().remove_dst(input_id);
        }

        Ok(())
    }

    pub fn propagate_input_key_change(
        &mut self,
        input_id: SoundInputId,
        states_changed: GridSpan,
        operation: StateOperation,
    ) {
        let input_data = self.sound_inputs.get(&input_id).unwrap();
        if let Some(pid) = input_data.target {
            self.modify_states_recursively(pid, states_changed, input_id, operation);
        }
    }

    pub fn add_number_source(&mut self, handle: NumberSourceHandle) {
        // TODO
        panic!()
    }

    pub fn remove_number_source(&mut self, id: NumberSourceId) {
        // TODO
        panic!()
    }

    pub fn add_number_input(&mut self, id: NumberInputId, owner: NumberInputOwner) {
        // TODO
        panic!()
    }

    pub fn remove_number_input(&mut self, id: NumberInputId) {
        // TODO
        panic!()
    }

    pub fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
    ) -> Result<(), NumberConnectionError> {
        // TODO
        panic!()
    }

    pub fn disconnect_number_input(
        &mut self,
        input_id: NumberInputId,
    ) -> Result<(), NumberConnectionError> {
        // TODO
        panic!()
    }

    fn describe(&self) -> SoundGraphDescription {
        let mut sound_processors = HashMap::<SoundProcessorId, SoundProcessorDescription>::new();
        let mut sound_inputs = HashMap::<SoundInputId, SoundInputDescription>::new();
        for proc_data in self.sound_processors.values() {
            sound_processors.insert(
                proc_data.id(),
                SoundProcessorDescription::new(
                    proc_data.id(),
                    proc_data.wrapper.is_static(),
                    proc_data.inputs.clone(),
                ),
            );
        }
        for input_data in self.sound_inputs.values() {
            sound_inputs.insert(
                input_data.id(),
                SoundInputDescription::new(
                    input_data.id(),
                    input_data.input().options(),
                    input_data.input().num_keys(),
                    input_data.target,
                ),
            );
        }
        SoundGraphDescription::new(sound_processors, sound_inputs)
    }

    pub fn run(&mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        let chunk_duration = Duration::from_micros((1_000_000.0 / chunks_per_sec) as u64);

        set_current_thread_priority(ThreadPriority::Max).unwrap();

        for p in self.sound_processors.values() {
            p.sound_processor().on_start_processing();
        }

        let mut deadline = Instant::now() + chunk_duration;

        loop {
            self.process_audio();
            if let PlaybackStatus::Stop = self.flush_messages() {
                println!("SoundEngine stopping");
                break;
            }

            let now = Instant::now();
            if now > deadline {
                println!("WARNING: SoundEngine missed a deadline");
            } else {
                let delta = deadline.duration_since(now);
                spin_sleep::sleep(delta);
            }
            deadline += chunk_duration;
        }

        for p in self.sound_processors.values() {
            p.sound_processor().on_stop_processing();
        }
    }

    pub fn flush_messages(&mut self) -> PlaybackStatus {
        let mut status = PlaybackStatus::Continue;
        loop {
            let msg = match self.receiver.try_recv() {
                Ok(msg) => msg,
                Err(e) => {
                    return match e {
                        TryRecvError::Empty => status,
                        TryRecvError::Disconnected => PlaybackStatus::Stop,
                    }
                }
            };
            match msg {
                SoundEngineMessage::AddSoundProcessor(w, obr) => {
                    self.add_sound_processor(w);
                    obr.fulfill(Ok(()));
                }
                SoundEngineMessage::RemoveSoundProcessor(spid, obr) => {
                    self.remove_sound_processor(spid);
                    obr.fulfill(Ok(()));
                }
                SoundEngineMessage::AddSoundInput(w, spid) => {
                    self.add_sound_input(spid, w);
                }
                SoundEngineMessage::RemoveSoundInput(iid) => {
                    self.remove_sound_input(iid);
                }
                SoundEngineMessage::ConnectInput(siid, spid, obr) => {
                    let r = self.connect_sound_input(siid, spid);
                    obr.fulfill(r);
                }
                SoundEngineMessage::DisconnectInput(siid, obr) => {
                    let r = self.disconnect_sound_input(siid);
                    obr.fulfill(r);
                }
                SoundEngineMessage::AddNumberInput(niid, nio) => {
                    self.add_number_input(niid, nio);
                }
                SoundEngineMessage::RemoveNumberInput(niid) => {
                    self.remove_number_input(niid);
                }
                SoundEngineMessage::AddNumberSource(h) => {
                    self.add_number_source(h);
                }
                SoundEngineMessage::RemoveNumberSource(nsid) => {
                    self.remove_number_source(nsid);
                }
                SoundEngineMessage::ConnectNumberInput(niid, nsid, obr) => {
                    let r = self.connect_number_input(niid, nsid);
                    obr.fulfill(r);
                }
                SoundEngineMessage::DisconnectNumberInput(niid, obr) => {
                    let r = self.disconnect_number_input(niid);
                    obr.fulfill(r);
                }
                SoundEngineMessage::Stop => {
                    status = PlaybackStatus::Stop;
                }
            }
        }
    }

    fn process_audio(&mut self) {
        // TODO
        // - place all static sound processors into a queue
        // - until the queue is empty:
        //    - remove the next static sound processor from the front of the queue
        //    - if it depends on any other static processors, put in on the back of the queue and continue
        //    - otherwise, invoke the processor directly.
        // **Mmmmm**: cache the order in which to invoke static processors and update it when changing
        //            the graph
        let mut buf = SoundChunk::new();
        for (id, pd) in &self.sound_processors {
            if pd.wrapper.is_static() {
                let stack = vec![SoundStackFrame::Processor(SoundProcessorFrame {
                    id: *id,
                    state_index: 0,
                })];
                // NOTE: starting with an empty stack here means that upstream
                // number sources will all be out of scope. It's probably safe
                // to allow upstream number sources as long as they are on a
                // unique path
                let context = Context::new(&self.sound_processors, &self.sound_inputs, stack);
                pd.wrapper.process_audio(&mut buf, context);
                // TODO: cache the static processor's output
            }
        }
    }
}

pub struct SoundEngineTools<'a> {
    soundengine: &'a mut SoundEngine,
}

impl<'a> SoundEngineTools<'a> {
    pub fn propagate_input_key_change(
        &mut self,
        input_id: SoundInputId,
        states_changed: GridSpan,
        operation: StateOperation,
    ) {
        self.soundengine
            .propagate_input_key_change(input_id, states_changed, operation);
    }
}
