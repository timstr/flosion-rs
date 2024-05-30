use std::hash::Hasher;

use crate::core::{
    graph::graphobject::GraphObjectHandle,
    revision::revision::{Revision, RevisionNumber, Versioned, VersionedHashMap},
    sound::{soundgrapherror::SoundError, soundnumbersource::SoundNumberSourceOwner},
};

use super::{
    soundgraph::SoundGraph,
    soundgraphdata::{
        SoundInputData, SoundNumberInputData, SoundNumberSourceData, SoundProcessorData,
    },
    soundgraphid::{SoundGraphId, SoundObjectId},
    soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId,
    soundprocessor::SoundProcessorId,
};

#[derive(Copy, Clone, Eq, PartialEq)]
enum SoundConnectionPart {
    Processor(SoundProcessorId),
    Input(SoundInputId),
}

/// A set of sound processors, all their constituent sound inputs, number
/// sources, and number inputs, and the relationships and connections
/// between them. Unlike the full SoundGraph, SoundGraphTopology is only
/// concerned with storing individual graph components and representing
/// their hierarchies and dependencies. Changes made to SoundGraphTopology
/// are fairly low-level and only basic error checking is done with each
/// edit, e.g. to confirm that entities referenced by the requested ids
/// actually exist. For higher level operations and more thorough error
/// checking, use SoundGraph instead.
#[derive(Clone)]
pub(crate) struct SoundGraphTopology {
    sound_processors: VersionedHashMap<SoundProcessorId, SoundProcessorData>,
    sound_inputs: VersionedHashMap<SoundInputId, SoundInputData>,
    number_sources: VersionedHashMap<SoundNumberSourceId, SoundNumberSourceData>,
    number_inputs: VersionedHashMap<SoundNumberInputId, SoundNumberInputData>,
}

impl SoundGraphTopology {
    /// Constructs a new instance without any processors or other components
    pub(crate) fn new() -> SoundGraphTopology {
        SoundGraphTopology {
            sound_processors: VersionedHashMap::new(),
            sound_inputs: VersionedHashMap::new(),
            number_sources: VersionedHashMap::new(),
            number_inputs: VersionedHashMap::new(),
        }
    }

    /// Access the set of sound processors stored in the topology
    pub(crate) fn sound_processors(
        &self,
    ) -> &VersionedHashMap<SoundProcessorId, SoundProcessorData> {
        &self.sound_processors
    }

    /// Access the set of sound inputs stored in the topology
    pub(crate) fn sound_inputs(&self) -> &VersionedHashMap<SoundInputId, SoundInputData> {
        &self.sound_inputs
    }

    /// Access the set of sound number sources stored in the topology
    pub(crate) fn number_sources(
        &self,
    ) -> &VersionedHashMap<SoundNumberSourceId, SoundNumberSourceData> {
        &self.number_sources
    }

    /// Access the set of sound number inputs stored in the topology
    pub(crate) fn number_inputs(
        &self,
    ) -> &VersionedHashMap<SoundNumberInputId, SoundNumberInputData> {
        &self.number_inputs
    }

    /// Look up a specific sound processor by its id
    pub(crate) fn sound_processor(
        &self,
        id: SoundProcessorId,
    ) -> Option<&Versioned<SoundProcessorData>> {
        self.sound_processors.get(&id)
    }

    /// Look up a specific sound processor by its id with mutable access
    pub(crate) fn sound_processor_mut(
        &mut self,
        id: SoundProcessorId,
    ) -> Option<&mut Versioned<SoundProcessorData>> {
        self.sound_processors.get_mut(&id)
    }

    /// Look up a specific sound input by its id
    pub(crate) fn sound_input(&self, id: SoundInputId) -> Option<&Versioned<SoundInputData>> {
        self.sound_inputs.get(&id)
    }

    /// Look up a specific sound number source by its id
    pub(crate) fn number_source(
        &self,
        id: SoundNumberSourceId,
    ) -> Option<&Versioned<SoundNumberSourceData>> {
        self.number_sources.get(&id)
    }

    /// Look up a specific sound number input by its id
    pub(crate) fn number_input(
        &self,
        id: SoundNumberInputId,
    ) -> Option<&Versioned<SoundNumberInputData>> {
        self.number_inputs.get(&id)
    }

    /// Look up a specific sound number input by its id with mutable access
    pub(crate) fn number_input_mut(
        &mut self,
        id: SoundNumberInputId,
    ) -> Option<&mut Versioned<SoundNumberInputData>> {
        self.number_inputs.get_mut(&id)
    }

    /// Returns an iterator listing all the sound inputs that are connected
    /// to the given sound processor, if any.
    pub(crate) fn sound_processor_targets<'a>(
        &'a self,
        id: SoundProcessorId,
    ) -> impl 'a + Iterator<Item = SoundInputId> {
        self.sound_inputs.values().filter_map(move |i| {
            if i.target() == Some(id) {
                Some(i.id())
            } else {
                None
            }
        })
    }

    /// Returns an iterator over all sound number inputs and their connected
    /// targets which would rely on data from the audio processing call stack
    /// that is made available through the given sound input.
    pub(crate) fn number_connection_crossings<'a>(
        &'a self,
        id: SoundInputId,
    ) -> impl 'a + Iterator<Item = (SoundNumberInputId, SoundNumberSourceId)> {
        self.number_inputs.values().flat_map(move |ni_data| {
            let id = id;
            ni_data
                .target_mapping()
                .items()
                .values()
                .filter_map(move |target_ns| {
                    let target_ns_owner = self.number_source(*target_ns).unwrap().owner();
                    let target_part = match target_ns_owner {
                        SoundNumberSourceOwner::SoundProcessor(spid) => {
                            SoundConnectionPart::Processor(spid)
                        }
                        SoundNumberSourceOwner::SoundInput(siid) => {
                            SoundConnectionPart::Input(siid)
                        }
                    };
                    if self.depends_on(target_part, SoundConnectionPart::Input(id))
                        && self.depends_on(
                            SoundConnectionPart::Input(id),
                            SoundConnectionPart::Processor(ni_data.owner()),
                        )
                    {
                        Some((ni_data.id(), *target_ns))
                    } else {
                        None
                    }
                })
        })
    }

    /// Look up a graph object by its id and return a handle to it.
    ///
    /// NOTE that currently the only graph objects are sound processors.
    /// This may be expanded upon in the future.
    pub(crate) fn graph_object(
        &self,
        object_id: SoundObjectId,
    ) -> Option<GraphObjectHandle<SoundGraph>> {
        match object_id {
            SoundObjectId::Sound(spid) => self
                .sound_processors
                .get(&spid)
                .map(|p| p.instance_arc().as_graph_object()),
        }
    }

    /// Returns an iterator over the ids of all graph objects in the topology.
    ///
    /// NOTE that currently the only graph objects are sound processors.
    /// This may be expanded upon in the future.
    pub(crate) fn graph_object_ids<'a>(&'a self) -> impl 'a + Iterator<Item = SoundObjectId> {
        let sound_objects = self.sound_processors.values().map(|x| x.id().into());
        sound_objects
    }

    /// Add a sound processsor to the topology.
    /// The provided SoundProcessorData must be empty (i.e. it must have
    /// no sound inputs, number sources, or number inputs) and its id must
    /// not yet be in use.
    pub(crate) fn add_sound_processor(
        &mut self,
        data: SoundProcessorData,
    ) -> Result<(), SoundError> {
        if !(data.sound_inputs().is_empty()
            && data.number_sources().is_empty()
            && data.number_inputs().is_empty())
        {
            return Err(SoundError::BadProcessorInit(data.id()));
        }
        if self.sound_processors.contains_key(&data.id()) {
            return Err(SoundError::ProcessorIdTaken(data.id()));
        }
        let prev = self.sound_processors.insert(data.id(), data);
        debug_assert!(prev.is_none());
        Ok(())
    }

    /// Remove a sound processor from the topology. The sound
    /// processor must not have any sound inputs, number sources,
    /// or number inputs associated with it.
    pub(crate) fn remove_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        let data = self
            .sound_processor(processor_id)
            .ok_or(SoundError::ProcessorNotFound(processor_id))?;

        if !(data.sound_inputs().is_empty()
            && data.number_sources().is_empty()
            && data.number_inputs().is_empty())
        {
            return Err(SoundError::BadProcessorCleanup(processor_id));
        }

        self.sound_processors.remove(&processor_id).unwrap();

        Ok(())
    }

    /// Add a sound input to the topology. The provided SoundInputData
    /// must have no number sources, its id must not yet be in use, and
    /// the sound processor to which it belongs must exist.
    pub(crate) fn add_sound_input(&mut self, data: SoundInputData) -> Result<(), SoundError> {
        if !data.number_sources().is_empty() {
            return Err(SoundError::BadSoundInputInit(data.id()));
        }
        if self.sound_inputs.contains_key(&data.id()) {
            return Err(SoundError::SoundInputIdTaken(data.id()));
        }
        let processor_id = data.owner();
        let proc_data = self
            .sound_processors
            .get_mut(&processor_id)
            .ok_or(SoundError::ProcessorNotFound(processor_id))?;
        debug_assert!(!proc_data.sound_inputs().contains(&data.id()));
        proc_data.sound_inputs_mut().push(data.id());
        let prev = self.sound_inputs.insert(data.id(), data);
        debug_assert!(prev.is_none());
        Ok(())
    }

    /// Remove a sound input from the topology.
    pub(crate) fn remove_sound_input(
        &mut self,
        input_id: SoundInputId,
        owner: SoundProcessorId,
    ) -> Result<(), SoundError> {
        let input_data = self
            .sound_inputs
            .get(&input_id)
            .ok_or(SoundError::SoundInputNotFound(input_id))?;

        if input_data.target().is_some() {
            return Err(SoundError::BadSoundInputCleanup(input_id));
        }

        if !input_data.number_sources().is_empty() {
            return Err(SoundError::BadSoundInputCleanup(input_id));
        }

        // remove the input from its owner
        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        debug_assert!(proc_data.sound_inputs().contains(&input_id));
        proc_data.sound_inputs_mut().retain(|iid| *iid != input_id);

        // remove the input
        self.sound_inputs.remove(&input_id).unwrap();

        Ok(())
    }

    /// Connect the given sound input to the given sound processor.
    /// Both the input and the processor must exist and the input
    /// must be unoccupied. No additional checks are performed.
    /// It is possible to create cycles using this method, even
    /// though cycles are generally not permitted. It is also
    /// possible to invalidate existing number inputs that rely
    /// on state from higher up the audio call stack by creating
    /// a separate pathway through which that state is not available.
    /// For additional error checking, use SoundGraph instead or see
    /// find_sound_error
    pub(crate) fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        if !self.sound_processors.contains_key(&processor_id) {
            return Err(SoundError::ProcessorNotFound(processor_id));
        }
        if !self.sound_inputs.contains_key(&input_id) {
            return Err(SoundError::SoundInputNotFound(input_id));
        }
        let input_data = self.sound_inputs.get_mut(&input_id).unwrap();
        if let Some(current_target) = input_data.target() {
            return Err(SoundError::SoundInputOccupied {
                input_id,
                current_target,
            });
        }
        input_data.set_target(Some(processor_id));
        Ok(())
    }

    /// Disconnect the given sound input from the processor it points to.
    /// The sound input must exist and it must be pointing to a sound
    /// processor already. No additional error checking is performed. It
    /// is possible to invalidate number sources which rely on state from
    /// higher up the audio call stack by removing their access to that
    /// state. For additional error checking, use SoundGraph instead or
    /// see find_sound_error.
    pub(crate) fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), SoundError> {
        let input_data = self
            .sound_inputs
            .get_mut(&input_id)
            .ok_or(SoundError::SoundInputNotFound(input_id))?;
        if input_data.target().is_none() {
            return Err(SoundError::SoundInputUnoccupied(input_id));
        }
        input_data.set_target(None);
        Ok(())
    }

    /// Add a sound number source to the topology. The number source's
    /// id must not be in use yet and its owner (i.e. the sound processor
    /// or input to which it belongs) must already exist.
    pub(crate) fn add_number_source(
        &mut self,
        data: SoundNumberSourceData,
    ) -> Result<(), SoundError> {
        // TODO: rename "sound number source" to something less vague.
        // Make it clear that it exposes audio processing state to
        // the numeric side of things. Perhaps SoundStateReader,
        // SoundStateAccessor, etc
        let id = data.id();
        let owner = data.owner();

        if self.number_sources.contains_key(&id) {
            return Err(SoundError::NumberSourceIdTaken(id));
        }

        match owner {
            SoundNumberSourceOwner::SoundProcessor(spid) => {
                let proc_data = self
                    .sound_processors
                    .get_mut(&spid)
                    .ok_or(SoundError::ProcessorNotFound(spid))?;
                debug_assert!(!proc_data.number_sources().contains(&id));
                proc_data.number_sources_mut().push(id);
            }
            SoundNumberSourceOwner::SoundInput(siid) => {
                let input_data = self
                    .sound_inputs
                    .get_mut(&siid)
                    .ok_or(SoundError::SoundInputNotFound(siid))?;
                debug_assert!(!input_data.number_sources().contains(&id));
                input_data.number_sources_mut().push(id);
            }
        }

        let prev = self.number_sources.insert(id, data);
        debug_assert!(prev.is_none());

        Ok(())
    }

    /// Remove a sound number source from the topology.
    pub(crate) fn remove_number_source(
        &mut self,
        source_id: SoundNumberSourceId,
        // TODO: owner here is redundant
        owner: SoundNumberSourceOwner,
    ) -> Result<(), SoundError> {
        if !self.number_sources.contains_key(&source_id) {
            return Err(SoundError::NumberSourceNotFound(source_id));
        }

        // remove the number source from its owner, if any
        match owner {
            SoundNumberSourceOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                proc_data
                    .number_sources_mut()
                    .retain(|iid| *iid != source_id);
            }
            SoundNumberSourceOwner::SoundInput(siid) => {
                let input_data = self.sound_inputs.get_mut(&siid).unwrap();
                input_data
                    .number_sources_mut()
                    .retain(|iid| *iid != source_id);
            }
        }

        // remove the number source from any number inputs that use it
        // TODO: or don't????
        for ni_data in self.number_inputs.values_mut() {
            let (numbergraph, mapping) = ni_data.number_graph_and_mapping_mut();
            if mapping.target_graph_input(source_id).is_some() {
                mapping.remove_target(source_id, numbergraph);
            }
        }

        // remove the number source
        self.number_sources.remove(&source_id).unwrap();

        Ok(())
    }

    /// Add a sound number input to the topology. The number input's
    /// id must not yet be in use and it must not yet be connected
    /// to any sound number sources in its input mapping. The sound
    /// processor to which the input belongs must exist.
    pub(crate) fn add_number_input(
        &mut self,
        data: SoundNumberInputData,
    ) -> Result<(), SoundError> {
        let id = data.id();

        if self.number_inputs.contains_key(&id) {
            return Err(SoundError::NumberInputIdTaken(id));
        }

        if !data.target_mapping().items().is_empty() {
            return Err(SoundError::BadNumberInputInit(id));
        }

        let proc_data = self
            .sound_processors
            .get_mut(&data.owner())
            .ok_or(SoundError::ProcessorNotFound(data.owner()))?;
        debug_assert!(!proc_data.number_inputs().contains(&id));

        proc_data.number_inputs_mut().push(id);

        let prev = self.number_inputs.insert(id, data);
        debug_assert!(prev.is_none());

        Ok(())
    }

    /// Remove a sound number input from the topology.
    pub(crate) fn remove_number_input(
        &mut self,
        id: SoundNumberInputId,
        owner: SoundProcessorId,
    ) -> Result<(), SoundError> {
        self.number_inputs
            .remove(&id)
            .ok_or(SoundError::NumberInputNotFound(id))?;

        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        proc_data.number_inputs_mut().retain(|niid| *niid != id);

        Ok(())
    }

    /// Add a sound number source as an input to the given sound number input.
    pub fn connect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) -> Result<(), SoundError> {
        let numberinputdata = self
            .number_input_mut(input_id)
            .ok_or(SoundError::NumberInputNotFound(input_id))?;
        let (numbergraph, mapping) = numberinputdata.number_graph_and_mapping_mut();
        mapping.add_target(source_id, numbergraph);
        Ok(())
    }

    pub fn disconnect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) -> Result<(), SoundError> {
        let numberinputdata = self
            .number_input_mut(input_id)
            .ok_or(SoundError::NumberInputNotFound(input_id))?;
        let (numbergraph, mapping) = numberinputdata.number_graph_and_mapping_mut();
        mapping.remove_target(source_id, numbergraph);
        Ok(())
    }

    /// Check whether the entity referred to by the given id exists in the topology
    pub fn contains(&self, graph_id: SoundGraphId) -> bool {
        match graph_id {
            SoundGraphId::SoundInput(siid) => self.sound_inputs.contains_key(&siid),
            SoundGraphId::SoundProcessor(spid) => self.sound_processors.contains_key(&spid),
            SoundGraphId::SoundNumberInput(niid) => self.number_inputs.contains_key(&niid),
            SoundGraphId::SoundNumberSource(nsid) => self.number_sources.contains_key(&nsid),
        }
    }

    /// Check whether one sound processor or input directly or indirectly is connected
    /// to another.
    fn depends_on(&self, part: SoundConnectionPart, other_part: SoundConnectionPart) -> bool {
        if part == other_part {
            return true;
        }
        match part {
            SoundConnectionPart::Processor(spid) => {
                for siid in self.sound_processor(spid).unwrap().sound_inputs() {
                    if self.depends_on(SoundConnectionPart::Input(*siid), other_part) {
                        return true;
                    }
                }
                false
            }
            SoundConnectionPart::Input(siid) => {
                if let Some(target) = self.sound_input(siid).unwrap().target() {
                    self.depends_on(SoundConnectionPart::Processor(target), other_part)
                } else {
                    false
                }
            }
        }
    }
}

impl Revision for SoundGraphTopology {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u64(self.sound_processors.get_revision().value());
        hasher.write_u64(self.sound_inputs.get_revision().value());
        hasher.write_u64(self.number_sources.get_revision().value());
        hasher.write_u64(self.number_inputs.get_revision().value());
        RevisionNumber::new(hasher.finish())
    }
}
