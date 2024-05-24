use std::{
    collections::HashSet,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use thread_priority::{set_current_thread_priority, ThreadPriority};

use crate::core::{
    engine::{
        garbage::GarbageDisposer,
        soundengine::{create_sound_engine, SoundEngineInterface, StopButton},
    },
    graph::{graph::Graph, graphobject::ObjectInitialization},
    jit::server::{JitClient, JitServer, JitServerBuilder},
    revision::revision::{Revision, RevisionNumber},
    uniqueid::IdGenerator,
};

use super::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgraphdata::{SoundNumberInputData, SoundProcessorData},
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

/// A reference to a subset of the parts of a SoundGraph
/// using their ids.
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
        niid: SoundNumberInputId,
        nsid: SoundNumberSourceId,
    ) -> bool {
        if self.number_inputs.contains(&niid) {
            return true;
        }
        if self.number_sources.contains(&nsid) {
            return true;
        }
        false
    }
}

/// A network of connected sound processors which are processing
/// audio in real time. SoundGraph combines both the individual
/// sound graph components living inside its SoundGraphTopology
/// with a SoundEngine instance which conducts audio processing
/// for those components.
///
/// Use a SoundGraph instance's methods to make changes to the
/// sound graph, such as adding, connecting, and removing sound
/// processors and their components. These changes will first be
/// validated locally for immediate error handling feedback.
/// Next, the changes will persist in local copy of the topology
/// which is available for immediate inspection at all times.
/// Finally, all edits are sent asynchronously to the SoundEngine
/// thread where the changes can be heard.
///
/// Currently, two additional threads are involved. A bookkeeping
/// thread is spawned which performs various asynchronous duties,
/// including receiving edits from the SoundGraph instance and
/// compiling them for the SoundEngine on the separate high-priority
/// audio thread. The SoundEngine maintains a StateGraph instance,
/// which is a compiled artefact representing a directly executable
/// version of the sound graph. Thus, the bookkeeping thread compiles
/// SoundGraphEdits into StateGraphEdits. Other chores that the
/// bookkeeping thread does include running the JIT compiler to
/// produce executable number inputs and taking out the garbage,
/// i.e. disposing of resources that could potentially block the
/// audio thread.
pub struct SoundGraph {
    local_topology: SoundGraphTopology,
    last_revision: Option<RevisionNumber>,

    engine_interface_thread: Option<JoinHandle<()>>,
    stop_button: StopButton,
    topology_sender: SyncSender<(SoundGraphTopology, Instant)>,
    jit_client: JitClient,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    number_source_idgen: IdGenerator<SoundNumberSourceId>,
    number_input_idgen: IdGenerator<SoundNumberInputId>,
}

impl SoundGraph {
    /// Constructs a new SoundGraph, and spawns an additional pair of
    /// threads for housekeeping and audio processing. Audio processing
    /// begins right away.
    pub fn new() -> SoundGraph {
        let topo_channel_size = 1024;
        let (topo_sender, topo_receiver) = sync_channel(topo_channel_size);
        let stop_button = StopButton::new();
        let stop_button_also = stop_button.clone();

        let (jit_server_builder, jit_client) = JitServerBuilder::new();

        // Thread for storing the inkwell (LLVM) context and containing
        // its heavily-referenced lifetime. Housekeeping is done directly
        // on this thread.
        let engine_interface_thread = std::thread::spawn(move || {
            let inkwell_context = inkwell::context::Context::create();

            // Construct the sound engine and its companion interfaces
            let (engine_interface, engine, garbage_disposer) =
                create_sound_engine(&stop_button_also);

            // Spawn a scoped thread to run the sound engine on a dedicated
            // high-priority thread while it borrows the inkwell context
            std::thread::scope(|scope| {
                scope.spawn(move || {
                    set_current_thread_priority(ThreadPriority::Max).unwrap();
                    engine.run();
                });
            });

            // Build the jit server which will compile number inputs for
            // the sound engine
            let jit_server = jit_server_builder.build_server(&inkwell_context);

            // Do the housekeeping chores until iterrupted
            Self::housekeeping_loop(
                jit_server,
                topo_receiver,
                engine_interface,
                garbage_disposer,
                stop_button_also,
            );
        });

        SoundGraph {
            local_topology: SoundGraphTopology::new(),

            engine_interface_thread: Some(engine_interface_thread),
            last_revision: None,
            stop_button,
            topology_sender: topo_sender,

            jit_client,

            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            number_source_idgen: IdGenerator::new(),
            number_input_idgen: IdGenerator::new(),
        }
    }

    fn housekeeping_loop<'ctx>(
        jit_server: JitServer<'ctx>,
        topo_receiver: Receiver<(SoundGraphTopology, Instant)>,
        mut engine_interface: SoundEngineInterface<'ctx>,
        garbage_disposer: GarbageDisposer,
        stop_button: StopButton,
    ) {
        // NOTE: both the engine interface and the garbage disposer
        // deal with LLVM resources and so need to stay on the same
        // thread as the inkwell_context above
        // Yes, it might be more efficient to perform topology diffing
        // and recompilation on the sound graph / ui thread but it
        // needs to happen on the same thread as the inkwell context,
        // whose lifetime needs to be confined here.
        // Yes, it might also seem safer and simpler to perform garbage
        // disposal on a separate thread, removing the need for the
        // following interleaving mess, but this would mean disposing
        // LLVM resources on a separate thread, which is also not allowed.
        loop {
            'handle_pending_updates: loop {
                garbage_disposer.clear();
                let mut issued_late_warning = false;
                // TODO: should throughput of jit_server be regulated here?
                jit_server.serve_pending_requests(engine_interface.current_topology());
                // handle at most a limited number of topology updates
                // to guarantee throughput for the garbage disposer
                for _ in 0..16 {
                    let topo = match topo_receiver.try_recv() {
                        Ok((topo, time_sent)) => {
                            let time_received = Instant::now();
                            let latency: Duration = time_received - time_sent;
                            let latency_ms = latency.as_millis();
                            if latency_ms > 200 && !issued_late_warning {
                                println!(
                                    "Warning: sound graph updates are {} milliseconds late",
                                    latency_ms
                                );
                                issued_late_warning = true;
                            }
                            topo
                        }
                        Err(TryRecvError::Empty) => {
                            break 'handle_pending_updates;
                        }
                        Err(TryRecvError::Disconnected) => {
                            println!(
                                "Sound topology update channel disconnected, \
                                         sound engine interface thread is exiting"
                            );
                            return;
                        }
                    };
                    if engine_interface.update(topo, &jit_server).is_err() {
                        println!(
                            "Failed to update sound engine, sound engine interface \
                                    thread is exiting"
                        );
                        return;
                    }
                }
            }

            if stop_button.was_stopped() {
                return;
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Access the sound graph topology. This is a local copy
    /// which is always up to date with respect to the latest
    /// edits that were applied to this sound graph instance.
    /// To modify the topology, see the various other high-level
    /// editing methods.
    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        &self.local_topology
    }

    /// Access the sound graph's jit client, e.g. to
    /// find and execute jit-compiled functions outside
    /// of the audio thread.
    pub(crate) fn jit_client(&self) -> &JitClient {
        &self.jit_client
    }

    /// Add a static sound processor to the sound graph,
    /// i.e. a sound processor which always has a single
    /// instance running in realtime and cannot be replicated.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectArchive.
    pub fn add_static_sound_processor<T: StaticSoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<StaticSoundProcessorHandle<T>, ()> {
        let id = self.sound_processor_idgen.next_id();
        let (add_time, time_nsid) =
            SoundGraphEdit::add_processor_time(id, &mut self.number_source_idgen);
        let mut edit_queue = Vec::new();
        let processor;
        {
            let tools = self.make_tools_for(id, &mut edit_queue);
            let p = T::new(tools, init)?;
            processor = Arc::new(StaticSoundProcessorWithId::new(p, id, time_nsid));
        }
        let processor2 = Arc::clone(&processor);
        let data = SoundProcessorData::new(processor);
        edit_queue.insert(0, SoundGraphEdit::Sound(SoundEdit::AddSoundProcessor(data)));
        edit_queue.insert(1, add_time);
        self.try_make_edits(edit_queue).unwrap();
        Ok(StaticSoundProcessorHandle::new(processor2))
    }

    /// Add a dynamic sound processor to the sound graph,
    /// i.e. a sound processor which is replicated for each
    /// input it is connected to, which are run on-demand.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectArchive.
    pub fn add_dynamic_sound_processor<T: DynamicSoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<DynamicSoundProcessorHandle<T>, ()> {
        let id = self.sound_processor_idgen.next_id();
        let (add_time, time_nsid) =
            SoundGraphEdit::add_processor_time(id, &mut self.number_source_idgen);
        let mut edit_queue = Vec::new();
        let processor;
        {
            let tools = self.make_tools_for(id, &mut edit_queue);
            let p = T::new(tools, init)?;
            processor = Arc::new(DynamicSoundProcessorWithId::new(p, id, time_nsid));
        }
        let processor2 = Arc::clone(&processor);
        let data = SoundProcessorData::new(processor);
        edit_queue.insert(0, SoundGraphEdit::Sound(SoundEdit::AddSoundProcessor(data)));
        edit_queue.insert(1, add_time);
        self.try_make_edits(edit_queue).unwrap();
        Ok(DynamicSoundProcessorHandle::new(processor2))
    }

    /// Connect a sound processor to a sound input. The processor
    /// and input must exist, the input must be unoccupied, and
    /// the connection must be valid, otherwise an Err is returned.
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

    /// Disconnect a sound input from the processor connected to it.
    /// The input must exist and must be connected to a sound processor.
    /// Additionally, there must be no number connections spanning the
    /// sound input, as these would be invalidated. Otherwise, an err
    /// is returned.
    pub fn disconnect_sound_input(&mut self, input_id: SoundInputId) -> Result<(), SoundError> {
        let mut edit_queue = Vec::new();
        edit_queue.push(SoundGraphEdit::Sound(SoundEdit::DisconnectSoundInput(
            input_id,
        )));
        self.try_make_edits(edit_queue)
    }

    // TODO: ??? Why is there no connect_number_input?
    // Why is one half implicit but this is explicit?
    // Maybe it's time to rethink number connections
    // and allow them to dangle such that they self-heal
    // once reconnected, rather than trashing all references
    // to out-of-scope number sources once a sound input
    // is broken?
    pub fn disconnect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) -> Result<(), SoundError> {
        self.try_make_change(|topo| {
            topo.disconnect_number_input(input_id, source_id);
            Ok(())
        })
    }

    /// Remove a sound processor completely from the sound graph.
    /// Any sound connections that include the processor and
    /// any number connections that include its components or
    /// span its sound inputs are disconnected.
    pub fn remove_sound_processor(&mut self, id: SoundProcessorId) -> Result<(), SoundError> {
        self.remove_objects_batch(&[id.into()])
    }

    /// Remove a set of top-level sound graph objects simultaneously.
    /// Sound and number connections which include or span the selected
    /// objects are disconnected before the objects are removed completely.
    /// This is more efficient than removing the objects sequentially.
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

        // TODO: remove any number connections that would be indirectly invalidated?
        // See also note at `disconnect_number_input` above, maybe they should
        // just be allowed to dangle

        // remove number connections
        for ni in self.local_topology.number_inputs().values() {
            for target_ns in ni.target_mapping().items().values() {
                if closure.includes_number_connection(ni.id(), *target_ns) {
                    edit_queue
                        .push(SoundNumberEdit::DisconnectNumberInput(ni.id(), *target_ns).into());
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
            edit_queue.push(SoundNumberEdit::RemoveNumberInput(*niid, owner).into());
        }

        // remove all number sources
        for nsid in &closure.number_sources {
            let owner = self.local_topology.number_source(*nsid).unwrap().owner();
            edit_queue.push(SoundNumberEdit::RemoveNumberSource(*nsid, owner).into());
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

    /// Create a SoundProcessorTools instance for making topological
    /// changes to the given sound processor and pass the tools to the
    /// provided closure. This is useful, for example, for example,
    /// for modifying sound inputs and number inputs and sources after
    /// the sound processor has been created.
    pub fn with_processor_tools<F: FnOnce(SoundProcessorTools)>(
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

    /// Make changes to a number input using the given closure,
    /// which is passed a mutable instance of the input's
    /// SoundNumberInputData.
    pub fn edit_number_input<R, F: FnOnce(&mut SoundNumberInputData) -> R>(
        &mut self,
        input_id: SoundNumberInputId,
        f: F,
    ) -> Result<R, SoundError> {
        self.try_make_change(|topo| {
            let number_input = topo
                .number_input_mut(input_id)
                .ok_or_else(|| SoundError::NumberInputNotFound(input_id))?;

            let r = f(number_input);

            if let Some(e) = find_error(topo) {
                Err(e)
            } else {
                Ok(r)
            }
        })
    }

    /// Internal helper method for building sound processor
    /// tools for the given processor. The sound graph instance
    /// is borrowed from mutably, and can't be used until the
    /// returned tools are dropped.
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

    /// Internal helper method for applying a list of SoundGraphEdits,
    /// checking for errors, rolling back on failure, and committing
    /// on success. Updates are NOT sent to the audio thread yet.
    /// Call flush_updates() to send an update to the audio thread.
    fn try_make_edits(&mut self, edit_queue: Vec<SoundGraphEdit>) -> Result<(), SoundError> {
        // TODO: if the SoundEngineInterface internally just receives
        // the updated topology directly and diffs it fully, why even
        // bother at all with SoundGraphEdits? Maybe they should be
        // purged if it simplifies other things too.
        self.try_make_change(|topo| {
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
        })
    }

    /// Internal helper method for modifying the topology locally,
    /// checking for any errors, rolling back on failure, and
    /// committing to the audio thread on success. Updates are NOT
    /// ent to the audio thread yet. Call flush_updates() to send
    /// an update to the audio thread.
    fn try_make_change<R, F: FnOnce(&mut SoundGraphTopology) -> Result<R, SoundError>>(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        debug_assert_eq!(find_error(&self.local_topology), None);
        let prev_topology = self.local_topology.clone();
        let res = f(&mut self.local_topology);
        if res.is_err() {
            self.local_topology = prev_topology;
        }
        debug_assert_eq!(find_error(&self.local_topology), None);
        res
    }

    /// Send any pending updates to the audio thread. Until this
    /// method is called, all edits are applied locally only.
    /// This method should thus be called whenever changes have
    /// been made that you would like to hear from the audio
    /// thread, and no more often than need in order to minimize
    /// thread communication and possible disruptions to the
    /// audio thread while it consumes updates.
    pub fn flush_updates(&mut self) {
        let revision = self.local_topology.get_revision();
        if self.last_revision == Some(revision) {
            return;
        }

        debug_assert_eq!(find_error(&self.local_topology), None);

        let time_sent = Instant::now();
        if let Err(err) = self
            .topology_sender
            .try_send((self.local_topology.clone(), time_sent))
        {
            match err {
                TrySendError::Full(_) => panic!("Sound Engine update overflow!"),
                TrySendError::Disconnected(_) => panic!("Sound Engine is no longer running!"),
            }
        }

        self.last_revision = Some(revision);
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
