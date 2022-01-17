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
    connectionerror::{ConnectionError, NumberConnectionError, SoundConnectionError},
    context::{Context, SoundProcessorFrame, SoundStackFrame},
    gridspan::GridSpan,
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceHandle, NumberSourceId},
    resultfuture::OutboundResult,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundgraphdescription::{
        NumberInputDescription, NumberSourceDescription, SoundGraphDescription,
        SoundInputDescription, SoundProcessorDescription,
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
    AddSoundProcessor {
        processor: Arc<dyn SoundProcessorWrapper>,
        result: OutboundResult<(), ()>,
    },
    RemoveSoundProcessor {
        processor_id: SoundProcessorId,
        result: OutboundResult<(), ()>,
    },
    AddSoundInput {
        input: Arc<dyn SoundInputWrapper>,
        owner: SoundProcessorId,
        result: OutboundResult<(), ()>,
    },
    RemoveSoundInput {
        input_id: SoundInputId,
        result: OutboundResult<(), ()>,
    },

    ConnectSoundInput {
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
        result: OutboundResult<(), ConnectionError>,
    },
    DisconnectSoundInput {
        input_id: SoundInputId,
        result: OutboundResult<(), ConnectionError>,
    },

    AddNumberSource {
        source: NumberSourceHandle,
        result: OutboundResult<(), ()>,
    },
    RemoveNumberSource {
        input_id: NumberSourceId,
        result: OutboundResult<(), ()>,
    },
    AddNumberInput {
        input: NumberInputId,
        owner: NumberInputOwner,
        result: OutboundResult<(), ()>,
    },
    RemoveNumberInput {
        input_id: NumberInputId,
        result: OutboundResult<(), ()>,
    },

    ConnectNumberInput {
        input_id: NumberInputId,
        target_id: NumberSourceId,
        result: OutboundResult<(), NumberConnectionError>,
    },
    DisconnectNumberInput {
        input_id: NumberInputId,
        result: OutboundResult<(), NumberConnectionError>,
    },

    Stop {
        result: OutboundResult<(), ()>,
    },
}

pub struct SoundEngine {
    sound_processors: HashMap<SoundProcessorId, EngineSoundProcessorData>,
    sound_inputs: HashMap<SoundInputId, EngineSoundInputData>,
    number_sources: HashMap<NumberSourceId, EngineNumberSourceData>,
    number_inputs: HashMap<NumberInputId, EngineNumberInputData>,
    receiver: Receiver<SoundEngineMessage>,
    static_processor_cache: Vec<(SoundProcessorId, Option<SoundChunk>)>,
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
                static_processor_cache: Vec::new(),
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
        self.update_static_processor_cache();
    }

    fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // TODO
        panic!();
        self.update_static_processor_cache();
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
        panic!();
        self.update_static_processor_cache();
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
        debug_assert!(desc.find_error().is_none());

        if let Some(err) = desc.add_connection(input_id, processor_id) {
            return Err(err.into());
        }

        if let Some(err) = desc.find_error() {
            return Err(err);
        }

        let input_data = self.sound_inputs.get_mut(&input_id);
        if input_data.is_none() {
            return Err(SoundConnectionError::InputNotFound(input_id).into());
        }
        let input_data = input_data.unwrap();
        if let Some(pid) = input_data.target {
            if pid == processor_id {
                return Err(SoundConnectionError::NoChange.into());
            }
            return Err(SoundConnectionError::InputOccupied {
                input_id,
                current_target: pid,
            }
            .into());
        }
        input_data.target = Some(processor_id);

        {
            let proc_data = self.sound_processors.get_mut(&processor_id);
            if proc_data.is_none() {
                return Err(SoundConnectionError::ProcessorNotFound(processor_id).into());
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

        self.update_static_processor_cache();

        Ok(())
    }

    fn disconnect_sound_input(&mut self, input_id: SoundInputId) -> Result<(), ConnectionError> {
        let mut desc = self.describe();
        debug_assert!(desc.find_error().is_none());

        if let Some(err) = desc.remove_connection(input_id) {
            return Err(err.into());
        }

        if let Some(err) = desc.find_error() {
            return Err(err.into());
        }

        let input_data = self.sound_inputs.get_mut(&input_id);
        if input_data.is_none() {
            return Err(SoundConnectionError::InputNotFound(input_id).into());
        }
        let input_data = input_data.unwrap();
        let processor_id = match input_data.target {
            Some(pid) => pid,
            None => return Err(SoundConnectionError::NoChange.into()),
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
                return Err(SoundConnectionError::ProcessorNotFound(processor_id).into());
            }
            let proc_data = proc_data.unwrap();
            proc_data.sound_processor().remove_dst(input_id);
        }

        self.update_static_processor_cache();

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
        let mut sound_inputs = HashMap::<SoundInputId, SoundInputDescription>::new();
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
        let mut number_sources = HashMap::<NumberSourceId, NumberSourceDescription>::new();
        // TODO
        let mut number_inputs = HashMap::<NumberInputId, NumberInputDescription>::new();
        // TODO
        SoundGraphDescription::new(
            sound_processors,
            sound_inputs,
            number_sources,
            number_inputs,
        )
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
                SoundEngineMessage::AddSoundProcessor { processor, result } => {
                    self.add_sound_processor(processor);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::RemoveSoundProcessor {
                    processor_id,
                    result,
                } => {
                    self.remove_sound_processor(processor_id);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::AddSoundInput {
                    input,
                    owner,
                    result,
                } => {
                    self.add_sound_input(owner, input);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::RemoveSoundInput { input_id, result } => {
                    self.remove_sound_input(input_id);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::ConnectSoundInput {
                    input_id,
                    processor_id,
                    result,
                } => {
                    let r = self.connect_sound_input(input_id, processor_id);
                    result.fulfill(r);
                }
                SoundEngineMessage::DisconnectSoundInput { input_id, result } => {
                    let r = self.disconnect_sound_input(input_id);
                    result.fulfill(r);
                }
                SoundEngineMessage::AddNumberInput {
                    input,
                    owner,
                    result,
                } => {
                    self.add_number_input(input, owner);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::RemoveNumberInput { input_id, result } => {
                    self.remove_number_input(input_id);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::AddNumberSource { source, result } => {
                    self.add_number_source(source);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::RemoveNumberSource { input_id, result } => {
                    self.remove_number_source(input_id);
                    result.fulfill(Ok(()));
                }
                SoundEngineMessage::ConnectNumberInput {
                    input_id,
                    target_id,
                    result,
                } => {
                    let r = self.connect_number_input(input_id, target_id);
                    result.fulfill(r);
                }
                SoundEngineMessage::DisconnectNumberInput { input_id, result } => {
                    let r = self.disconnect_number_input(input_id);
                    result.fulfill(r);
                }
                SoundEngineMessage::Stop { result } => {
                    status = PlaybackStatus::Stop;
                    result.fulfill(Ok(()));
                }
            }
        }
    }

    fn update_static_processor_cache(&mut self) {
        // TODO: topological sort of all static processors
        // efficiency is not paramount
        let mut remaining_static_proc_ids: Vec<SoundProcessorId> = self
            .sound_processors
            .values()
            .filter_map(|proc_data| {
                if proc_data.wrapper.is_static() {
                    Some(proc_data.id())
                } else {
                    None
                }
            })
            .collect();
        fn depends_on_remaining_procs(
            proc_id: SoundProcessorId,
            remaining: &Vec<SoundProcessorId>,
            engine: &SoundEngine,
        ) -> bool {
            let proc_data = engine.sound_processors.get(&proc_id).unwrap();
            for input_id in &proc_data.inputs {
                let input_data = engine.sound_inputs.get(&input_id).unwrap();
                if let Some(target_proc_id) = input_data.target {
                    if remaining
                        .iter()
                        .find(|pid| **pid == target_proc_id)
                        .is_some()
                    {
                        return true;
                    }
                    if depends_on_remaining_procs(target_proc_id, remaining, engine) {
                        return true;
                    }
                }
            }
            return false;
        }

        self.static_processor_cache.clear();

        loop {
            let next_avail_proc = remaining_static_proc_ids.iter().position(|pid| {
                !depends_on_remaining_procs(*pid, &remaining_static_proc_ids, &self)
            });
            match next_avail_proc {
                Some(idx) => {
                    let pid = remaining_static_proc_ids.remove(idx);
                    self.static_processor_cache.push((pid, None))
                }
                None => break,
            }
        }
    }

    fn process_audio(&mut self) {
        debug_assert!(
            self.static_processor_cache
                .iter()
                .find(|(pid, _)| self.sound_processors.get(pid).is_none())
                .is_none(),
            "The cached static processor ids should all exist"
        );
        debug_assert!(
            self.sound_processors
                .iter()
                .filter_map(|(pid, pdata)| if pdata.wrapper.is_static() {
                    Some(*pid)
                } else {
                    None
                })
                .find(|pid| self
                    .static_processor_cache
                    .iter()
                    .find(|(i, _)| *i == *pid)
                    .is_none())
                .is_none(),
            "All static processors should be in the cache"
        );

        for (_, ch) in &mut self.static_processor_cache {
            *ch = None;
        }

        for idx in 0..self.static_processor_cache.len() {
            let pid = self.static_processor_cache[idx].0;
            let proc_data = self.sound_processors.get(&pid).unwrap();
            debug_assert!(proc_data.wrapper.is_static());
            let stack = vec![SoundStackFrame::Processor(SoundProcessorFrame {
                id: pid,
                state_index: 0,
            })];
            // NOTE: starting with an empty stack here means that upstream
            // number sources will all be out of scope. It's probably safe
            // to allow upstream number sources as long as they are on a
            // unique path
            let context = Context::new(
                &self.sound_processors,
                &self.sound_inputs,
                &self.static_processor_cache,
                stack,
            );
            let mut chunk = SoundChunk::new();
            proc_data.wrapper.process_audio(&mut chunk, context);
            self.static_processor_cache[idx].1 = Some(chunk);
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
