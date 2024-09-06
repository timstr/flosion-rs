use crate::ui_core::arguments::ParsedArguments;

use super::{
    expression::SoundExpressionId,
    sounderror::SoundError,
    soundgraphdata::SoundExpressionData,
    soundgraphid::SoundObjectId,
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::find_sound_error,
    soundinput::SoundInputId,
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, SoundProcessorId, StaticSoundProcessor,
        StaticSoundProcessorHandle,
    },
    soundprocessortools::SoundProcessorTools,
    topologyedits::{
        build_dynamic_sound_processor, build_static_sound_processor, SoundGraphIdGenerators,
    },
};

// TODO: consider replacing SoundGraph with/renaming SoundGraphTopology
pub struct SoundGraph {
    local_topology: SoundGraphTopology,

    id_generators: SoundGraphIdGenerators,
}

impl SoundGraph {
    /// Constructs a new SoundGraph, and spawns an additional pair of
    /// threads for housekeeping and audio processing. Audio processing
    /// begins right away.
    pub fn new() -> SoundGraph {
        SoundGraph {
            local_topology: SoundGraphTopology::new(),

            id_generators: SoundGraphIdGenerators::new(),
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

    /// Add a static sound processor to the sound graph,
    /// i.e. a sound processor which always has a single
    /// instance running in realtime and cannot be replicated.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_static_sound_processor<T: 'static + StaticSoundProcessor>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<StaticSoundProcessorHandle<T>, SoundError> {
        self.try_make_change(move |topo, idgens| {
            build_static_sound_processor::<T>(topo, idgens, args)
        })
    }

    /// Add a dynamic sound processor to the sound graph,
    /// i.e. a sound processor which is replicated for each
    /// input it is connected to, which are run on-demand.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_dynamic_sound_processor<T: 'static + DynamicSoundProcessor>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<DynamicSoundProcessorHandle<T>, SoundError> {
        self.try_make_change(move |topo, idgens| {
            build_dynamic_sound_processor::<T>(topo, idgens, args)
        })
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
}
