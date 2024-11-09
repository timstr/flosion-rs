use std::collections::{HashMap, HashSet};

use hashstash::{
    stash_clone_with_context, InplaceUnstasher, ObjectHash, Order, Stash, Stashable, Stasher,
    UnstashError, Unstashable, UnstashableInplace, Unstasher,
};

use crate::{
    core::stashing::{StashingContext, UnstashingContext},
    ui_core::{arguments::ParsedArguments, factories::Factories},
};

use super::{
    sounderror::SoundError,
    soundgraphid::{SoundGraphComponentLocation, SoundObjectId},
    soundgraphvalidation::find_sound_error,
    soundinput::{BasicProcessorInput, SoundInputLocation},
    soundprocessor::{AnySoundProcessor, SoundProcessorId},
};

pub struct SoundGraph {
    sound_processors: HashMap<SoundProcessorId, Box<dyn AnySoundProcessor>>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        SoundGraph {
            sound_processors: HashMap::new(),
        }
    }

    /// Access the set of sound processors stored in the graph
    pub(crate) fn sound_processors(
        &self,
    ) -> &HashMap<SoundProcessorId, Box<dyn AnySoundProcessor>> {
        &self.sound_processors
    }

    /// Look up a specific sound processor by its id
    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&dyn AnySoundProcessor> {
        match self.sound_processors.get(&id) {
            Some(p) => Some(&**p),
            None => None,
        }
    }

    pub(crate) fn sound_processor_mut(
        &mut self,
        id: SoundProcessorId,
    ) -> Option<&mut dyn AnySoundProcessor> {
        match self.sound_processors.get_mut(&id) {
            Some(p) => Some(&mut **p),
            None => None,
        }
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
    pub fn add_sound_processor(&mut self, processor: Box<dyn AnySoundProcessor>) {
        let prev = self.sound_processors.insert(processor.id(), processor);
        debug_assert!(prev.is_none());
    }

    pub fn remove_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        // Disconnect all inputs from the processor
        for proc_data in self.sound_processors.values_mut() {
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
    pub(crate) fn connect_sound_input(
        &mut self,
        input_location: SoundInputLocation,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        let Some(proc) = self.sound_processor_mut(input_location.processor()) else {
            return Err(SoundError::ProcessorNotFound(input_location.processor()));
        };
        proc.with_input_mut(input_location.input(), |input| {
            input.set_target(Some(processor_id));
        })
        .ok_or(SoundError::SoundInputNotFound(input_location))
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
        let Some(proc) = self.sound_processor_mut(input_location.processor()) else {
            return Err(SoundError::ProcessorNotFound(input_location.processor()));
        };
        proc.with_input_mut(input_location.input(), |input| {
            input.set_target(None);
        })
        .ok_or(SoundError::SoundInputNotFound(input_location))
    }

    /// Check whether the entity referred to by the given id exists in the graph
    pub fn contains<I: Into<SoundGraphComponentLocation>>(&self, location: I) -> bool {
        let location: SoundGraphComponentLocation = location.into();
        let mut exists = false;
        match location {
            SoundGraphComponentLocation::Processor(spid) => {
                exists = self.sound_processors.contains_key(&spid);
            }
            SoundGraphComponentLocation::Input(sil) => {
                self.sound_processor(sil.processor())
                    .map(|p| p.with_input(sil.input(), |_| exists = true));
            }
            SoundGraphComponentLocation::Expression(el) => {
                self.sound_processor(el.processor())
                    .map(|p| p.with_expression(el.expression(), |_| exists = true));
            }
            SoundGraphComponentLocation::ProcessorArgument(pal) => {
                self.sound_processor(pal.processor())
                    .map(|p| p.with_processor_argument(pal.argument(), |_| exists = true));
            }
        }
        exists
    }

    pub fn with_sound_input<R, F: FnMut(&BasicProcessorInput) -> R>(
        &self,
        location: SoundInputLocation,
        f: F,
    ) -> Option<R> {
        let Some(proc_data) = self.sound_processors.get(&location.processor()) else {
            return None;
        };
        proc_data.with_input(location.input(), f)
    }

    pub fn with_sound_input_mut<R, F: FnMut(&mut BasicProcessorInput) -> R>(
        &mut self,
        location: SoundInputLocation,
        f: F,
    ) -> Option<R> {
        let Some(proc_data) = self.sound_processors.get_mut(&location.processor()) else {
            return None;
        };
        proc_data.with_input_mut(location.input(), f)
    }

    /// Helper method for editing the sound graph, detecting errors,
    /// rolling back the changes if any errors were found, and otherwise
    /// keeping the change.
    pub fn try_make_change<R, F: FnOnce(&mut SoundGraph) -> Result<R, SoundError>>(
        &mut self,
        stash: &Stash,
        factories: &Factories,
        f: F,
    ) -> Result<R, SoundError> {
        if let Err(e) = self.validate() {
            panic!(
                "Called try_make_change() on a SoundGraph which is already invalid: {:?}",
                e.explain(self)
            );
        }

        let (previous_graph, stash_handle) = stash_clone_with_context(
            self,
            stash,
            StashingContext::new_stashing_normally(),
            UnstashingContext::new(factories),
        )
        .unwrap();

        debug_assert_eq!(
            stash_handle.object_hash(),
            ObjectHash::from_stashable_and_context(self, StashingContext::new_stashing_normally()),
            "SoundGraph hash differs after stash-cloning"
        );

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

impl Stashable<StashingContext> for SoundGraph {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.array_of_proxy_objects(
            self.sound_processors.values(),
            |processor, stasher| {
                // id (needed during in-place unstashing to find existing objects)
                stasher.u64(processor.id().value() as _);

                // type name (needed for factory during unstashing)
                stasher.string(processor.as_graph_object().get_dynamic_type().name());

                // contents
                stasher.object_proxy(|stasher| processor.stash(stasher));
            },
            Order::Unordered,
        );
    }
}

impl Stashable<()> for SoundGraph {
    fn stash(&self, stasher: &mut Stasher<()>) {
        // TODO: don't duplicate this, find a way to delegate
        // without indirection
        stasher.array_of_proxy_objects_with_context(
            self.sound_processors.values(),
            |processor, stasher| {
                // id (needed during in-place unstashing to find existing objects)
                stasher.u64(processor.id().value() as _);

                // type name (needed for factory during unstashing)
                stasher.string(processor.as_graph_object().get_dynamic_type().name());

                // contents
                stasher.object_proxy(|stasher| processor.stash(stasher));
            },
            Order::Unordered,
            StashingContext::new_stashing_normally(),
        );
    }
}

impl Unstashable<UnstashingContext<'_>> for SoundGraph {
    fn unstash(unstasher: &mut Unstasher<UnstashingContext>) -> Result<SoundGraph, UnstashError> {
        let mut graph = SoundGraph::new();
        unstasher.array_of_proxy_objects(|unstasher| {
            // id
            let id = SoundProcessorId::new(unstasher.u64()? as _);

            // type name
            let type_name = unstasher.string()?;

            let mut processor = unstasher
                .context()
                .factories()
                .sound_objects()
                .create(&type_name, &ParsedArguments::new_empty())
                .into_boxed_sound_processor()
                .unwrap();

            // contents
            unstasher.object_proxy_inplace(|unstasher| processor.unstash_inplace(unstasher))?;

            debug_assert_eq!(processor.id(), id);

            graph.add_sound_processor(processor);

            Ok(())
        })?;

        Ok(graph)
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for SoundGraph {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        let time_to_write = unstasher.time_to_write();

        let mut procs_to_keep: HashSet<SoundProcessorId> = HashSet::new();

        // nodes
        unstasher.array_of_proxy_objects(|unstasher| {
            // id
            let id = SoundProcessorId::new(unstasher.u64()? as _);

            // type name
            let type_name = unstasher.string()?;

            if let Some(existing_proc) = self.sound_processor_mut(id) {
                unstasher
                    .object_proxy_inplace(|unstasher| existing_proc.unstash_inplace(unstasher))?;
            } else {
                let mut proc = unstasher
                    .context()
                    .factories()
                    .sound_objects()
                    .create(&type_name, &ParsedArguments::new_empty())
                    .into_boxed_sound_processor()
                    .unwrap();

                // contents
                unstasher.object_proxy_inplace(|unstasher| proc.unstash_inplace(unstasher))?;

                if time_to_write {
                    self.add_sound_processor(proc);
                }
            }

            procs_to_keep.insert(id);

            Ok(())
        })?;

        if time_to_write {
            // remove processors which were not stashed
            self.sound_processors
                .retain(|id, _| procs_to_keep.contains(id));
        }

        Ok(())
    }
}
