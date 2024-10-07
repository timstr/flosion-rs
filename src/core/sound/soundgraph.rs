use std::{collections::HashMap, rc::Rc};

use hashstash::{Order, Stashable};

use crate::{core::uniqueid::IdGenerator, ui_core::arguments::ParsedArguments};

use super::{
    expression::ProcessorExpressionId,
    expressionargument::{ProcessorArgumentId, SoundInputArgumentId},
    sounderror::SoundError,
    soundgraphdata::SoundProcessorData,
    soundgraphid::{SoundGraphComponentLocation, SoundObjectId},
    soundgraphvalidation::find_sound_error,
    soundinput::{ProcessorInput, ProcessorInputId, SoundInputLocation},
    soundprocessor::{
        SoundProcessorId, WhateverSoundProcessor, WhateverSoundProcessorHandle,
        WhateverSoundProcessorWithId,
    },
    soundprocessortools::SoundProcessorTools,
};

#[derive(Clone)]
pub struct SoundGraph {
    sound_processors: HashMap<SoundProcessorId, SoundProcessorData>,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    input_idgen: IdGenerator<ProcessorInputId>,
    expression_idgen: IdGenerator<ProcessorExpressionId>,
    proc_arg_idgen: IdGenerator<ProcessorArgumentId>,
    input_arg_idgen: IdGenerator<SoundInputArgumentId>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        SoundGraph {
            sound_processors: HashMap::new(),
            sound_processor_idgen: IdGenerator::new(),
            input_idgen: IdGenerator::new(),
            expression_idgen: IdGenerator::new(),
            proc_arg_idgen: IdGenerator::new(),
            input_arg_idgen: IdGenerator::new(),
        }
    }

    /// Access the set of sound processors stored in the graph
    pub(crate) fn sound_processors(&self) -> &HashMap<SoundProcessorId, SoundProcessorData> {
        &self.sound_processors
    }

    /// Look up a specific sound processor by its id
    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&SoundProcessorData> {
        self.sound_processors.get(&id)
    }

    pub(crate) fn sound_processor_mut(
        &mut self,
        id: SoundProcessorId,
    ) -> Option<&mut SoundProcessorData> {
        self.sound_processors.get_mut(&id)
    }

    // TODO: rename to e.g. inputs_connected_to
    pub(crate) fn sound_processor_targets<'a>(
        &'a self,
        id: SoundProcessorId,
    ) -> Vec<SoundInputLocation> {
        let mut input_locations = Vec::new();
        for proc_data in self.sound_processors.values() {
            proc_data.foreach_input(|input, location| {
                if input.target() == Some(id) {
                    input_locations.push(location);
                }
            });
        }
        input_locations
    }

    /// Returns an iterator over the ids of all graph objects in the graph.
    ///
    /// NOTE that currently the only graph objects are sound processors.
    /// This may be expanded upon in the future.
    pub(crate) fn graph_object_ids<'a>(&'a self) -> impl 'a + Iterator<Item = SoundObjectId> {
        let sound_objects = self.sound_processors.values().map(|x| x.id().into());
        sound_objects
    }

    /// Add a static sound processor to the sound graph,
    /// i.e. a sound processor which always has a single
    /// instance running in realtime and cannot be replicated.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_sound_processor<T: 'static + WhateverSoundProcessor>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<WhateverSoundProcessorHandle<T>, SoundError> {
        let id = self.sound_processor_idgen.next_id();

        // The tools which the processor can use to give itself
        // new inputs, etc
        let tools = SoundProcessorTools::new(
            id,
            &mut self.input_idgen,
            &mut self.expression_idgen,
            &mut self.proc_arg_idgen,
            &mut self.input_arg_idgen,
        );

        // construct the actual processor instance by its
        // concrete type
        let processor = T::new(tools, args);

        // wrap the processor in a type-erased Rc
        let processor = Rc::new(WhateverSoundProcessorWithId::new(processor, id));
        let processor2 = Rc::clone(&processor);

        self.sound_processors
            .insert(id, SoundProcessorData::new(id, processor));

        Ok(WhateverSoundProcessorHandle::new(processor2))
    }

    pub fn remove_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        // Disconnect all inputs from the processor
        for proc_data in self.sound_processors.values() {
            proc_data.foreach_input_mut(|input, _| {
                if input.target() == Some(processor_id) {
                    input.set_target(None);
                }
            });
        }

        self.sound_processors.remove(&processor_id);

        Ok(())
    }

    /// Connect the given sound input to the given sound processor.
    /// Both the input and the processor must exist and the input
    /// must be unoccupied. No additional checks are performed.
    /// It is possible to create cycles using this method, even
    /// though cycles are generally not permitted. It is also
    /// possible to invalidate existing expression that rely
    /// on state from higher up the audio call stack by creating
    /// a separate pathway through which that state is not available.
    // TODO: remove?
    pub(crate) fn connect_sound_input(
        &mut self,
        input_location: SoundInputLocation,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        todo!()
    }

    /// Disconnect the given sound input from the processor it points to.
    /// The sound input must exist and it must be pointing to a sound
    /// processor already. No additional error checking is performed. It
    /// is possible to invalidate expression arguments which rely on state from
    /// higher up the audio call stack by removing their access to that
    /// state. For additional error checking, use SoundGraph instead or
    /// see find_sound_error.
    // TODO: remove?
    pub(crate) fn disconnect_sound_input(
        &mut self,
        input_location: SoundInputLocation,
    ) -> Result<(), SoundError> {
        todo!()
    }

    /// Check whether the entity referred to by the given id exists in the graph
    pub fn contains<I: Into<SoundGraphComponentLocation>>(&self, id: I) -> bool {
        todo!()
    }

    pub fn with_sound_input<R, F: FnMut(&ProcessorInput) -> R>(
        &self,
        location: SoundInputLocation,
        f: F,
    ) -> Option<R> {
        let Some(proc_data) = self.sound_processors.get(&location.processor()) else {
            return None;
        };
        proc_data.with_input(location.input(), f)
    }

    pub fn with_sound_input_mut<R, F: FnMut(&mut ProcessorInput) -> R>(
        &mut self,
        location: SoundInputLocation,
        f: F,
    ) -> Option<R> {
        let Some(proc_data) = self.sound_processors.get_mut(&location.processor()) else {
            return None;
        };
        proc_data.with_input_mut(location.input(), f)
    }

    /// Create a SoundProcessorTools instance for making
    /// changes to the given sound processor and pass the tools to the
    /// provided closure. This is useful, for example, for example,
    /// for modifying sound inputs and expressions and arguments after
    /// the sound processor has been created.
    pub fn with_processor_tools<R, F: FnOnce(SoundProcessorTools) -> Result<R, SoundError>>(
        &mut self,
        processor_id: SoundProcessorId,
        f: F,
    ) -> Result<R, SoundError> {
        if !self.sound_processors.contains_key(&processor_id) {
            return Err(SoundError::ProcessorNotFound(processor_id));
        }
        self.try_make_change(|graph| {
            let tools = SoundProcessorTools::new(
                processor_id,
                &mut graph.input_idgen,
                &mut graph.expression_idgen,
                &mut graph.proc_arg_idgen,
                &mut graph.input_arg_idgen,
            );
            f(tools)
        })
    }

    /// Helper method for editing the sound graph, detecting errors,
    /// rolling back the changes if any errors were found, and otherwise
    /// keeping the change.
    pub fn try_make_change<R, F: FnOnce(&mut SoundGraph) -> Result<R, SoundError>>(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        if let Err(e) = self.validate() {
            panic!(
                "Called try_make_change() on a SoundGraph which is already invalid: {:?}",
                e.explain(self)
            );
        }
        let previous_graph = self.clone();
        let res = f(self);
        if res.is_err() {
            *self = previous_graph;
            return res;
        } else if let Err(e) = self.validate() {
            *self = previous_graph;
            return Err(e);
        }
        res
    }

    pub fn validate(&self) -> Result<(), SoundError> {
        match find_sound_error(self) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Stashable for SoundGraph {
    fn stash(&self, stasher: &mut hashstash::Stasher) {
        // sound processors
        stasher.array_of_proxy_objects(
            self.sound_processors.values(),
            |proc_data, stasher| {
                stasher.u64(proc_data.id().value() as u64);
                // TODO: processor instance?
                // TODO: call visit() method to stash components?
            },
            Order::Unordered,
        );
    }
}
