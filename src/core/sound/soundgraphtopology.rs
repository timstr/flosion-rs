use std::{collections::HashSet, hash::Hasher};

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

#[derive(Clone)]
pub(crate) struct SoundGraphTopology {
    sound_processors: VersionedHashMap<SoundProcessorId, SoundProcessorData>,
    sound_inputs: VersionedHashMap<SoundInputId, SoundInputData>,
    number_sources: VersionedHashMap<SoundNumberSourceId, SoundNumberSourceData>,
    number_inputs: VersionedHashMap<SoundNumberInputId, SoundNumberInputData>,
}

impl SoundGraphTopology {
    pub(crate) fn new() -> SoundGraphTopology {
        SoundGraphTopology {
            sound_processors: VersionedHashMap::new(),
            sound_inputs: VersionedHashMap::new(),
            number_sources: VersionedHashMap::new(),
            number_inputs: VersionedHashMap::new(),
        }
    }

    pub(crate) fn sound_processors(
        &self,
    ) -> &VersionedHashMap<SoundProcessorId, SoundProcessorData> {
        &self.sound_processors
    }

    pub(crate) fn sound_inputs(&self) -> &VersionedHashMap<SoundInputId, SoundInputData> {
        &self.sound_inputs
    }

    pub(crate) fn number_sources(
        &self,
    ) -> &VersionedHashMap<SoundNumberSourceId, SoundNumberSourceData> {
        &self.number_sources
    }

    pub(crate) fn number_inputs(
        &self,
    ) -> &VersionedHashMap<SoundNumberInputId, SoundNumberInputData> {
        &self.number_inputs
    }

    pub(crate) fn sound_processor(
        &self,
        id: SoundProcessorId,
    ) -> Option<&Versioned<SoundProcessorData>> {
        self.sound_processors.get(&id)
    }

    pub(crate) fn sound_processor_mut(
        &mut self,
        id: SoundProcessorId,
    ) -> Option<&mut Versioned<SoundProcessorData>> {
        self.sound_processors.get_mut(&id)
    }

    pub(crate) fn sound_input(&self, id: SoundInputId) -> Option<&Versioned<SoundInputData>> {
        self.sound_inputs.get(&id)
    }

    pub(crate) fn number_source(
        &self,
        id: SoundNumberSourceId,
    ) -> Option<&Versioned<SoundNumberSourceData>> {
        self.number_sources.get(&id)
    }

    pub(crate) fn number_input(
        &self,
        id: SoundNumberInputId,
    ) -> Option<&Versioned<SoundNumberInputData>> {
        self.number_inputs.get(&id)
    }

    pub(crate) fn number_input_mut(
        &mut self,
        id: SoundNumberInputId,
    ) -> Option<&mut Versioned<SoundNumberInputData>> {
        self.number_inputs.get_mut(&id)
    }

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

    pub(crate) fn all_ids(&self) -> HashSet<SoundGraphId> {
        let mut ids: HashSet<SoundGraphId> = HashSet::new();
        ids.extend(
            self.sound_processors
                .keys()
                .map(|i| -> SoundGraphId { (*i).into() }),
        );
        ids.extend(
            self.sound_inputs
                .keys()
                .map(|i| -> SoundGraphId { (*i).into() }),
        );
        ids.extend(
            self.number_sources
                .keys()
                .map(|i| -> SoundGraphId { (*i).into() }),
        );
        ids.extend(
            self.number_inputs
                .keys()
                .map(|i| -> SoundGraphId { (*i).into() }),
        );
        ids
    }

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

    pub(crate) fn graph_object_ids<'a>(&'a self) -> impl 'a + Iterator<Item = SoundObjectId> {
        let sound_objects = self.sound_processors.values().map(|x| x.id().into());
        sound_objects
    }

    pub(crate) fn graph_objects<'a>(
        &'a self,
    ) -> impl 'a + Iterator<Item = GraphObjectHandle<SoundGraph>> {
        let sound_objects = self
            .sound_processors
            .values()
            .map(|x| x.instance_arc().as_graph_object());
        sound_objects
    }

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

    fn add_sound_input_key(&mut self, input_id: SoundInputId, index: usize) {
        let input_data = self.sound_inputs.get_mut(&input_id).unwrap();
        let n = input_data.num_keys();
        debug_assert!(index <= n);
        input_data.set_num_keys(n + 1);
    }

    fn remove_sound_input_key(&mut self, input_id: SoundInputId, index: usize) {
        let input_data = self.sound_inputs.get_mut(&input_id).unwrap();
        let n = input_data.num_keys();
        debug_assert!(index < n);
        input_data.set_num_keys(n - 1);
    }

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

    pub(crate) fn add_number_source(
        &mut self,
        data: SoundNumberSourceData,
    ) -> Result<(), SoundError> {
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

    pub(crate) fn add_number_input(
        &mut self,
        data: SoundNumberInputData,
    ) -> Result<(), SoundError> {
        let id = data.id();

        if self.number_inputs.contains_key(&id) {
            return Err(SoundError::NumberInputIdTaken(id));
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

    pub fn connect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) {
        let numberinputdata = self.number_input_mut(input_id).unwrap();
        let (numbergraph, mapping) = numberinputdata.number_graph_and_mapping_mut();
        mapping.add_target(source_id, numbergraph);
    }

    pub fn disconnect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) {
        let numberinputdata = self.number_input_mut(input_id).unwrap();
        let (numbergraph, mapping) = numberinputdata.number_graph_and_mapping_mut();
        mapping.remove_target(source_id, numbergraph);
    }

    pub fn contains(&self, graph_id: SoundGraphId) -> bool {
        match graph_id {
            SoundGraphId::SoundInput(siid) => self.sound_inputs.contains_key(&siid),
            SoundGraphId::SoundProcessor(spid) => self.sound_processors.contains_key(&spid),
            SoundGraphId::SoundNumberInput(niid) => self.number_inputs.contains_key(&niid),
            SoundGraphId::SoundNumberSource(nsid) => self.number_sources.contains_key(&nsid),
        }
    }

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
