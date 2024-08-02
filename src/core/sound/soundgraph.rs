use std::{
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use hashrevise::{Revisable, RevisionHash};
use thread_priority::{set_current_thread_priority, ThreadPriority};

use crate::core::{
    engine::{
        garbage::GarbageDisposer,
        soundengine::{create_sound_engine, SoundEngineInterface, StopButton},
    },
    graph::{graph::Graph, graphobject::ObjectInitialization},
    jit::server::{JitClient, JitServer, JitServerBuilder},
    uniqueid::IdGenerator,
};

use super::{
    expression::SoundExpressionId,
    expressionargument::{
        ProcessorTimeExpressionArgument, SoundExpressionArgumentId, SoundExpressionArgumentOwner,
    },
    soundgraphdata::{SoundExpressionArgumentData, SoundExpressionData, SoundProcessorData},
    soundgrapherror::SoundError,
    soundgraphid::SoundObjectId,
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::find_sound_error,
    soundinput::SoundInputId,
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
    },
    soundprocessortools::SoundProcessorTools,
};

/// Convenience struct for passing all sound graph id
/// generators around as a whole
pub(crate) struct SoundGraphIdGenerators {
    pub sound_processor: IdGenerator<SoundProcessorId>,
    pub sound_input: IdGenerator<SoundInputId>,
    pub expression_argument: IdGenerator<SoundExpressionArgumentId>,
    pub expression: IdGenerator<SoundExpressionId>,
}

impl SoundGraphIdGenerators {
    pub(crate) fn new() -> SoundGraphIdGenerators {
        SoundGraphIdGenerators {
            sound_processor: IdGenerator::new(),
            sound_input: IdGenerator::new(),
            expression_argument: IdGenerator::new(),
            expression: IdGenerator::new(),
        }
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
/// topology changes into StateGraphEdits. Other chores that the
/// bookkeeping thread does include running the JIT compiler to
/// produce executable expression and taking out the garbage,
/// i.e. disposing of resources that could potentially block the
/// audio thread.
pub struct SoundGraph {
    local_topology: SoundGraphTopology,
    last_revision: Option<RevisionHash>,

    engine_interface_thread: Option<JoinHandle<()>>,
    stop_button: StopButton,
    topology_sender: SyncSender<(SoundGraphTopology, Instant)>,
    jit_client: Option<JitClient>,

    id_generators: SoundGraphIdGenerators,
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

                // Build the jit server which will compile expression for
                // the sound engine
                let jit_server = jit_server_builder.build_server(&inkwell_context);

                // Do the housekeeping chores until interrupted
                Self::housekeeping_loop(
                    jit_server,
                    topo_receiver,
                    engine_interface,
                    garbage_disposer,
                    stop_button_also,
                );
            });
        });

        SoundGraph {
            local_topology: SoundGraphTopology::new(),

            engine_interface_thread: Some(engine_interface_thread),
            last_revision: None,
            stop_button,
            topology_sender: topo_sender,

            jit_client: Some(jit_client),

            id_generators: SoundGraphIdGenerators::new(),
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
        'housekeeping: loop {
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
                            break 'housekeeping;
                        }
                    };
                    if let Err(e) = engine_interface.update(topo, &jit_server) {
                        println!(
                            "Failed to update sound engine, housekeeping thread is exiting: {:?}",
                            e
                        );
                        break 'housekeeping;
                    }
                }
            }

            if stop_button.was_stopped() {
                break 'housekeeping;
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
        &self.jit_client.as_ref().unwrap()
    }

    /// Add a static sound processor to the sound graph,
    /// i.e. a sound processor which always has a single
    /// instance running in realtime and cannot be replicated.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_static_sound_processor<T: StaticSoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<StaticSoundProcessorHandle<T>, SoundError> {
        let id = self.id_generators.sound_processor.next_id();

        // Every sound processor gets a 'time' expression argument
        let time_data = SoundExpressionArgumentData::new(
            self.id_generators.expression_argument.next_id(),
            Arc::new(ProcessorTimeExpressionArgument::new(id)),
            SoundExpressionArgumentOwner::SoundProcessor(id),
        );

        let processor = self.try_make_change(move |topo, idgens| {
            // Add a new processor data item to the topology,
            // but without the processor instance. This allows
            // the processor's topology to be modified within
            // the processor's new() method, e.g. to add inputs.
            let data = SoundProcessorData::new_empty(id);
            topo.add_sound_processor(data)?;

            // The tools which the processor can use to give itself
            // new inputs, etc
            let tools = SoundProcessorTools::new(id, topo, idgens);

            // construct the actual processor instance by its
            // concrete type
            let processor = T::new(tools, init).map_err(|_| SoundError::BadProcessorInit(id))?;

            // wrap the processor in a type-erased Arc
            let processor = Arc::new(StaticSoundProcessorWithId::new(
                processor,
                id,
                time_data.id(),
            ));
            let processor2 = Arc::clone(&processor);

            // add the missing processor instance to the
            // newly created processor data in the topology
            topo.sound_processor_mut(id)
                .unwrap()
                .set_processor(processor);

            // Add the 'time' expression argument
            topo.add_expression_argument(time_data)?;

            Ok(processor2)
        })?;

        Ok(StaticSoundProcessorHandle::new(processor))
    }

    /// Add a dynamic sound processor to the sound graph,
    /// i.e. a sound processor which is replicated for each
    /// input it is connected to, which are run on-demand.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_dynamic_sound_processor<T: DynamicSoundProcessor>(
        &mut self,
        init: ObjectInitialization,
    ) -> Result<DynamicSoundProcessorHandle<T>, SoundError> {
        let id = self.id_generators.sound_processor.next_id();

        // Every sound processor gets a 'time' expression argument
        let time_data = SoundExpressionArgumentData::new(
            self.id_generators.expression_argument.next_id(),
            Arc::new(ProcessorTimeExpressionArgument::new(id)),
            SoundExpressionArgumentOwner::SoundProcessor(id),
        );

        let processor = self.try_make_change(move |topo, idgens| {
            // Add a new processor data item to the topology,
            // but without the processor instance. This allows
            // the processor's topology to be modified within
            // the processor's new() method, e.g. to add inputs.
            let data = SoundProcessorData::new_empty(id);
            topo.add_sound_processor(data)?;

            // The tools which the processor can use to give itself
            // new inputs, etc
            let tools = SoundProcessorTools::new(id, topo, idgens);

            // construct the actual processor instance by its
            // concrete type
            let processor = T::new(tools, init).map_err(|_| SoundError::BadProcessorInit(id))?;

            // wrap the processor in a type-erased Arc
            let processor = Arc::new(DynamicSoundProcessorWithId::new(
                processor,
                id,
                time_data.id(),
            ));
            let processor2 = Arc::clone(&processor);

            // add the missing processor instance to the
            // newly created processor data in the topology
            topo.sound_processor_mut(id)
                .unwrap()
                .set_processor(processor);

            // Add the 'time' expression argument
            topo.add_expression_argument(time_data)?;

            Ok(processor2)
        })?;

        Ok(DynamicSoundProcessorHandle::new(processor))
    }

    /// Connect a sound processor to a sound input. The processor
    /// and input must exist, the input must be unoccupied, and
    /// the connection must be valid, otherwise an Err is returned.
    pub fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        self.try_make_change(|topo, _| topo.connect_sound_input(input_id, processor_id))
    }

    /// Disconnect a sound input from the processor connected to it.
    /// The input must exist and must be connected to a sound processor.
    pub fn disconnect_sound_input(&mut self, input_id: SoundInputId) -> Result<(), SoundError> {
        self.try_make_change(|topo, _| topo.disconnect_sound_input(input_id))
    }

    /// Remove a sound processor completely from the sound graph.
    /// Any sound connections that include the processor and
    /// any expressions that include its components are disconnected.
    pub fn remove_sound_processor(&mut self, id: SoundProcessorId) -> Result<(), SoundError> {
        self.remove_objects_batch(&[id.into()])
    }

    /// Remove a set of top-level sound graph objects simultaneously.
    /// Sound connections which include or span the selected
    /// objects are disconnected before the objects are removed completely.
    /// This is more efficient than removing the objects sequentially.
    pub fn remove_objects_batch(&mut self, objects: &[SoundObjectId]) -> Result<(), SoundError> {
        self.try_make_change(|topo, _| {
            for id in objects {
                match id {
                    SoundObjectId::Sound(id) => {
                        Self::remove_sound_processor_and_components(*id, topo)?
                    }
                }
            }
            Ok(())
        })
    }

    /// Internal helper method for removing a sound processor and all
    /// of its constituents
    fn remove_sound_processor_and_components(
        processor_id: SoundProcessorId,
        topo: &mut SoundGraphTopology,
    ) -> Result<(), SoundError> {
        let mut expressions_to_remove = Vec::new();
        let mut expr_arguments_to_remove = Vec::new();
        let mut sound_inputs_to_remove = Vec::new();
        let mut sound_inputs_to_disconnect = Vec::new();

        let proc = topo
            .sound_processor(processor_id)
            .ok_or(SoundError::ProcessorNotFound(processor_id))?;

        for ni in proc.expressions() {
            expressions_to_remove.push(*ni);
        }

        for ns in proc.expression_arguments() {
            expr_arguments_to_remove.push(*ns);
        }

        for si in proc.sound_inputs() {
            sound_inputs_to_remove.push(*si);
            let input = topo.sound_input(*si).unwrap();
            for ns in input.expression_arguments() {
                expr_arguments_to_remove.push(*ns);
            }
            if topo.sound_input(*si).unwrap().target().is_some() {
                sound_inputs_to_disconnect.push(*si);
            }
        }

        for si in topo.sound_inputs().values() {
            if si.target() == Some(processor_id) {
                sound_inputs_to_disconnect.push(si.id());
            }
        }

        // ---

        for si in sound_inputs_to_disconnect {
            topo.disconnect_sound_input(si)?;
        }

        for ni in expressions_to_remove {
            topo.remove_expression(ni, processor_id)?;
        }

        for ns in expr_arguments_to_remove {
            topo.remove_expression_argument(ns)?;
        }

        for si in sound_inputs_to_remove {
            topo.remove_sound_input(si, processor_id)?;
        }

        topo.remove_sound_processor(processor_id)?;

        Ok(())
    }

    /// Create a SoundProcessorTools instance for making topological
    /// changes to the given sound processor and pass the tools to the
    /// provided closure. This is useful, for example, for example,
    /// for modifying sound inputs and expressions and arguments after
    /// the sound processor has been created.
    pub fn with_processor_tools<R, F: FnOnce(SoundProcessorTools) -> Result<R, SoundError>>(
        &mut self,
        processor_id: SoundProcessorId,
        f: F,
    ) -> Result<R, SoundError> {
        self.try_make_change(|topo, idgens| {
            let tools = SoundProcessorTools::new(processor_id, topo, idgens);
            f(tools)
        })
    }

    pub(crate) fn edit_topology<R, F: FnOnce(&mut SoundGraphTopology) -> Result<R, SoundError>>(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        self.try_make_change(|topo, _| f(topo))
    }

    /// Make changes to an expression using the given closure,
    /// which is passed a mutable instance of the input's
    /// SoundExpressionData.
    pub fn edit_expression<R, F: FnOnce(&mut SoundExpressionData) -> R>(
        &mut self,
        input_id: SoundExpressionId,
        f: F,
    ) -> Result<R, SoundError> {
        self.try_make_change(|topo, _| {
            let expr = topo
                .expression_mut(input_id)
                .ok_or(SoundError::ExpressionNotFound(input_id))?;

            let r = f(expr);

            if let Some(e) = find_sound_error(topo) {
                Err(e)
            } else {
                Ok(r)
            }
        })
    }

    /// Internal helper method for modifying the topology locally,
    /// checking for any errors, rolling back on failure, and
    /// committing to the audio thread on success. Updates are NOT
    /// sent to the audio thread yet. Call flush_updates() to send
    /// an update to the audio thread.
    fn try_make_change<
        R,
        F: FnOnce(&mut SoundGraphTopology, &mut SoundGraphIdGenerators) -> Result<R, SoundError>,
    >(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        debug_assert_eq!(find_sound_error(&self.local_topology), None);
        let prev_topology = self.local_topology.clone();
        let res = f(&mut self.local_topology, &mut self.id_generators);
        if res.is_err() {
            self.local_topology = prev_topology;
            return res;
        } else if let Some(e) = find_sound_error(&self.local_topology) {
            self.local_topology = prev_topology;
            return Err(e);
        }
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

        debug_assert_eq!(find_sound_error(&self.local_topology), None);

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
        // Press the stop button so the audio and housekeeping threads also
        // know to stop
        self.stop_button.stop();

        // drop the jit client to allow the jit server to exit.
        // The jit client accesses resources living on the jit server's
        // thread but subverts the use of lifetimes deliberately to allow
        // jit functions to be used more widely. Instead, to ensure safety,
        // the jit server blocks when dropped until the client is dropped first.
        // Dropping the client here before waiting on the engine interface (jit)
        // thread prevents a deadlock.
        std::mem::drop(self.jit_client.take().unwrap());

        let engine_interface_thread = self.engine_interface_thread.take().unwrap();
        engine_interface_thread.join().unwrap();
    }
}

impl Graph for SoundGraph {
    type ObjectId = SoundObjectId;
}
