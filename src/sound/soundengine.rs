use std::{
    collections::HashMap,
    ops::Add,
    sync::mpsc::TryRecvError,
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

use spin_sleep::LoopHelper;

use super::{
    connectionerror::ConnectionError,
    context::Context,
    gridspan::GridSpan,
    resultfuture::OutboundResult,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundgraphdescription::{
        SoundGraphDescription, SoundInputDescription, SoundProcessorDescription,
    },
    soundinput::{InputOptions, SoundInputId, SoundInputWrapper},
    soundprocessor::{SoundProcessorId, SoundProcessorWrapper},
};

#[derive(Copy, Clone)]
pub enum StateOperation {
    Insert,
    Erase,
}

pub struct SoundInputData {
    input: Box<dyn SoundInputWrapper>,
    target: Option<SoundProcessorId>,
    options: InputOptions, // TODO: this is redundant
    id: SoundInputId,      // TODO: this is redundant
}

impl SoundInputData {
    pub fn new(input: Box<dyn SoundInputWrapper>) -> SoundInputData {
        let id = input.id();
        let options = input.options();
        SoundInputData {
            input,
            target: None,
            options,
            id,
        }
    }

    pub fn options(&self) -> InputOptions {
        self.options
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }

    pub fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub fn input(&self) -> &dyn SoundInputWrapper {
        &*self.input
    }
}

pub struct SoundProcessorData {
    id: SoundProcessorId,
    wrapper: Box<dyn SoundProcessorWrapper>,
    inputs: Vec<SoundInputData>,
}

impl SoundProcessorData {
    pub fn new(
        wrapper: Box<dyn SoundProcessorWrapper>,
        id: SoundProcessorId,
    ) -> SoundProcessorData {
        let inputs = Vec::<SoundInputData>::new();

        SoundProcessorData {
            id,
            wrapper,
            inputs,
        }
    }

    pub fn inputs(&self) -> &Vec<SoundInputData> {
        &self.inputs
    }

    pub fn sound_processor(&self) -> &dyn SoundProcessorWrapper {
        &*self.wrapper
    }
}

pub enum SoundEngineMessage {
    AddSoundProcessor(
        SoundProcessorId,
        Box<dyn SoundProcessorWrapper>,
        OutboundResult<(), ()>,
    ),
    RemoveSoundProcessor(SoundProcessorId, OutboundResult<(), ()>),
    AddSoundInput(Box<dyn SoundInputWrapper>, SoundProcessorId),
    RemoveSoundInput(SoundInputId),
    ConnectInput(
        SoundInputId,
        SoundProcessorId,
        OutboundResult<(), ConnectionError>,
    ),
    DisconnectInput(SoundInputId, OutboundResult<(), ConnectionError>),
    Stop,
}

pub struct SoundEngine {
    processors: HashMap<SoundProcessorId, SoundProcessorData>,
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
                processors: HashMap::new(),
                receiver: rx,
            },
            tx,
        )
    }

    fn add_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
        wrapper: Box<dyn SoundProcessorWrapper>,
    ) {
        let data = SoundProcessorData::new(wrapper, processor_id);
        self.processors.insert(processor_id, data);
    }

    fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // TODO
        panic!()
    }

    fn add_sound_input(
        &mut self,
        processor_id: SoundProcessorId,
        wrapper: Box<dyn SoundInputWrapper>,
    ) {
        assert!(self
            .processors
            .iter()
            .find_map(|(_, pd)| pd.inputs.iter().find(|i| i.id == wrapper.id()))
            .is_none());
        let data = SoundInputData::new(wrapper);
        let pd = self.processors.get_mut(&processor_id).unwrap();
        pd.inputs.push(data);
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
        let outbound_connections: Vec<(SoundProcessorId, GridSpan, SoundInputId)>;
        {
            let proc_data = self.processors.get_mut(&proc_id).unwrap();
            let proc = &mut proc_data.wrapper;
            let gs = match operation {
                StateOperation::Insert => proc.insert_dst_states(dst_iid, dst_states),
                StateOperation::Erase => proc.erase_dst_states(dst_iid, dst_states),
            };
            if proc.is_static() {
                return;
            }
            outbound_connections = proc_data
                .inputs
                .iter_mut()
                .filter_map(|i| {
                    let gsi = match operation {
                        StateOperation::Insert => i.input.insert_states(gs),
                        StateOperation::Erase => i.input.erase_states(gs),
                    };
                    match i.target {
                        Some(t) => Some((t, gsi, i.id)),
                        None => None,
                    }
                })
                .collect();
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

        let input_proc_id: SoundProcessorId;
        {
            let (input, input_parent_id) =
                match self.processors.iter_mut().find_map(|(proc_id, proc)| {
                    assert_eq!(*proc_id, proc.id);
                    match proc.inputs.iter_mut().find(|i| i.id == input_id) {
                        Some(i) => Some((i, proc.id)),
                        None => None,
                    }
                }) {
                    Some(p) => p,
                    None => return Err(ConnectionError::InputNotFound),
                };
            if let Some(t) = input.target {
                return if t == processor_id {
                    Err(ConnectionError::NoChange)
                } else {
                    Err(ConnectionError::InputOccupied)
                };
            }
            input.target = Some(processor_id);
            input_proc_id = input_parent_id;
        }

        let input_proc_states: usize;
        {
            let wrapper = &self.processors.get(&input_proc_id).unwrap().wrapper;
            input_proc_states = wrapper.num_states();
        }

        {
            let proc_data = self.processors.get_mut(&processor_id).unwrap();
            let proc = &mut proc_data.wrapper;
            proc.add_dst(input_id);
        }

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

        let processor_id: SoundProcessorId;
        let input_proc_id: SoundProcessorId;
        {
            let found_input = self.processors.iter_mut().find_map(|(_, proc)| {
                match proc.inputs.iter_mut().find(|i| i.id == input_id) {
                    Some(i) => Some((i, proc.id)),
                    None => None,
                }
            });
            let (input, input_parent_id) = match found_input {
                Some(p) => p,
                None => return Err(ConnectionError::InputNotFound),
            };
            processor_id = match input.target {
                Some(t) => t,
                None => return Err(ConnectionError::NoChange),
            };
            input.target = None;
            input_proc_id = input_parent_id;
        }

        let input_proc_states: usize;
        {
            let wrapper = &self.processors.get(&input_proc_id).unwrap().wrapper;
            input_proc_states = wrapper.num_states();
        }
        self.modify_states_recursively(
            processor_id,
            GridSpan::new_contiguous(0, input_proc_states),
            input_id,
            StateOperation::Erase,
        );

        {
            let proc_data = self.processors.get_mut(&processor_id).unwrap();
            let proc = &mut proc_data.wrapper;
            proc.remove_dst(input_id);
        }

        Ok(())
    }

    pub fn propagate_input_key_change(
        &mut self,
        input_id: SoundInputId,
        states_changed: GridSpan,
        operation: StateOperation,
    ) {
        let input = self
            .processors
            .iter()
            .find_map(|(_, proc)| proc.inputs.iter().find(|i| i.id == input_id))
            .unwrap();
        if let Some(pid) = input.target {
            self.modify_states_recursively(pid, states_changed, input_id, operation);
        }
    }

    fn describe(&self) -> SoundGraphDescription {
        let mut processors = Vec::<SoundProcessorDescription>::new();
        for (proc_id, proc) in &self.processors {
            let mut inputs = Vec::<SoundInputDescription>::new();
            for i in &proc.inputs {
                let input_instance = &i.input;
                inputs.push(SoundInputDescription::new(
                    input_instance.id(),
                    i.options,
                    input_instance.num_keys(),
                    i.target,
                ))
            }
            processors.push(SoundProcessorDescription::new(
                *proc_id,
                proc.wrapper.is_static(),
                inputs,
            ))
        }
        SoundGraphDescription::new(processors)
    }

    pub fn run(&mut self) {
        let chunks_per_sec = (SAMPLE_FREQUENCY as f64) / (CHUNK_SIZE as f64);
        // let usec_per_chunk = (1_000_000.0 / chunks_per_sec) as u64;
        // let mut then = Instant::now();

        for p in self.processors.values() {
            p.sound_processor().on_start_processing();
        }

        let mut loop_helper = LoopHelper::builder()
            .report_interval_s(0.5)
            .build_with_target_rate(chunks_per_sec);

        loop {
            let _delta = loop_helper.loop_start();

            self.process_audio();
            if let PlaybackStatus::Stop = self.flush_messages() {
                println!("SoundEngine stopping");
                break;
            }

            if let Some(fps) = loop_helper.report_rate() {
                println!(
                    "Sound engine running at {} chunks per sec (expected {})",
                    fps, chunks_per_sec
                );
            }

            loop_helper.loop_sleep();
            // let next = then.add(Duration::from_micros(usec_per_chunk));
            // loop {
            //     let now = Instant::now();
            //     let elapsed = now.duration_since(then).as_micros() as u64;
            //     if elapsed >= usec_per_chunk {
            //         break;
            //     }
            //     // thread::park_timeout(Duration::from_micros(1000));
            //     std::hint::spin_loop();
            // }
            // then = next;
        }

        for p in self.processors.values() {
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
                SoundEngineMessage::AddSoundProcessor(spid, w, obr) => {
                    self.add_sound_processor(spid, w);
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
        for (id, pd) in &self.processors {
            if pd.wrapper.is_static() {
                let mut ctx = Context::new(Some(&mut buf), &self.processors, *id, 0);
                pd.wrapper.process_audio(&mut ctx);
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
