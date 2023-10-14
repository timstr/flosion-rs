use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
};

use crate::core::{
    graph::graphobject::GraphObjectHandle, revision::Revision,
    sound::soundnumbersource::SoundNumberSourceOwner,
};

use super::{
    soundedit::{SoundEdit, SoundNumberEdit},
    soundgraph::SoundGraph,
    soundgraphdata::{
        SoundInputData, SoundNumberInputData, SoundNumberSourceData, SoundProcessorData,
    },
    soundgraphedit::SoundGraphEdit,
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
    sound_processors: HashMap<SoundProcessorId, SoundProcessorData>,
    sound_inputs: HashMap<SoundInputId, SoundInputData>,
    number_sources: HashMap<SoundNumberSourceId, SoundNumberSourceData>,
    number_inputs: HashMap<SoundNumberInputId, SoundNumberInputData>,
}

impl SoundGraphTopology {
    pub(crate) fn new() -> SoundGraphTopology {
        SoundGraphTopology {
            sound_processors: HashMap::new(),
            sound_inputs: HashMap::new(),
            number_sources: HashMap::new(),
            number_inputs: HashMap::new(),
        }
    }

    pub(crate) fn sound_processors(&self) -> &HashMap<SoundProcessorId, SoundProcessorData> {
        &self.sound_processors
    }

    pub(crate) fn sound_inputs(&self) -> &HashMap<SoundInputId, SoundInputData> {
        &self.sound_inputs
    }

    pub(crate) fn number_sources(&self) -> &HashMap<SoundNumberSourceId, SoundNumberSourceData> {
        &self.number_sources
    }

    pub(crate) fn number_inputs(&self) -> &HashMap<SoundNumberInputId, SoundNumberInputData> {
        &self.number_inputs
    }

    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&SoundProcessorData> {
        self.sound_processors.get(&id)
    }

    pub(crate) fn sound_input(&self, id: SoundInputId) -> Option<&SoundInputData> {
        self.sound_inputs.get(&id)
    }

    pub(crate) fn number_source(&self, id: SoundNumberSourceId) -> Option<&SoundNumberSourceData> {
        self.number_sources.get(&id)
    }

    pub(crate) fn number_input(&self, id: SoundNumberInputId) -> Option<&SoundNumberInputData> {
        self.number_inputs.get(&id)
    }

    pub(crate) fn number_input_mut(
        &mut self,
        id: SoundNumberInputId,
    ) -> Option<&mut SoundNumberInputData> {
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

    pub(crate) fn make_sound_graph_edit(&mut self, edit: SoundGraphEdit) {
        match edit {
            SoundGraphEdit::Sound(e) => self.make_sound_edit(e),
            SoundGraphEdit::Number(e) => self.make_sound_number_edit(e),
        }
    }

    pub(crate) fn make_sound_edit(&mut self, edit: SoundEdit) {
        match edit {
            SoundEdit::AddSoundProcessor(data) => self.add_sound_processor(data),
            SoundEdit::RemoveSoundProcessor(id) => self.remove_sound_processor(id),
            SoundEdit::AddSoundInput(data) => self.add_sound_input(data),
            SoundEdit::RemoveSoundInput(id, owner) => self.remove_sound_input(id, owner),
            SoundEdit::AddSoundInputKey(siid, i) => self.add_sound_input_key(siid, i),
            SoundEdit::RemoveSoundInputKey(siid, i) => self.remove_sound_input_key(siid, i),
            SoundEdit::ConnectSoundInput(siid, spid) => self.connect_sound_input(siid, spid),
            SoundEdit::DisconnectSoundInput(siid) => self.disconnect_sound_input(siid),
        }
    }

    pub(crate) fn make_sound_number_edit(&mut self, edit: SoundNumberEdit) {
        match edit {
            SoundNumberEdit::AddNumberSource(data) => self.add_number_source(data),
            SoundNumberEdit::RemoveNumberSource(id, owner) => self.remove_number_source(id, owner),
            SoundNumberEdit::AddNumberInput(data) => self.add_number_input(data),
            SoundNumberEdit::RemoveNumberInput(id, owner) => self.remove_number_input(id, owner),
        }
    }

    fn add_sound_processor(&mut self, data: SoundProcessorData) {
        debug_assert!(data.sound_inputs().is_empty());
        debug_assert!(data.number_sources().is_empty());
        debug_assert!(data.number_inputs().is_empty());
        let prev = self.sound_processors.insert(data.id(), data);
        debug_assert!(prev.is_none());
    }

    fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        debug_assert!((|| {
            let data = self.sound_processor(processor_id).unwrap();
            data.sound_inputs().is_empty()
                && data.number_sources().is_empty()
                && data.number_inputs().is_empty()
        })());

        self.sound_processors.remove(&processor_id).unwrap();
    }

    fn add_sound_input(&mut self, data: SoundInputData) {
        debug_assert!(data.number_sources().is_empty());
        let processor_id = data.owner();
        let proc_data = self.sound_processors.get_mut(&processor_id).unwrap();
        proc_data.sound_inputs_mut().push(data.id());
        let prev = self.sound_inputs.insert(data.id(), data);
        debug_assert!(prev.is_none())
    }

    fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        debug_assert!({
            let input_data = self.sound_inputs.get(&input_id).unwrap();

            input_data.target().is_none() && input_data.number_sources().len() == 0
        });

        // remove the input from its owner
        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        debug_assert!(proc_data.sound_inputs().contains(&input_id));
        proc_data.sound_inputs_mut().retain(|iid| *iid != input_id);

        // remove the input
        self.sound_inputs.remove(&input_id).unwrap();
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

    fn connect_sound_input(&mut self, input_id: SoundInputId, processor_id: SoundProcessorId) {
        debug_assert!(self.sound_processors.contains_key(&processor_id));
        let input_data = self.sound_inputs.get_mut(&input_id).unwrap();
        debug_assert!(input_data.target().is_none());
        input_data.set_target(Some(processor_id));
    }

    fn disconnect_sound_input(&mut self, input_id: SoundInputId) {
        let input_data = self.sound_inputs.get_mut(&input_id).unwrap();
        debug_assert!(input_data.target().is_some());
        input_data.set_target(None);
    }

    fn add_number_source(&mut self, data: SoundNumberSourceData) {
        let id = data.id();
        let owner = data.owner();

        match owner {
            SoundNumberSourceOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                debug_assert!(!proc_data.number_sources().contains(&id));
                proc_data.number_sources_mut().push(id);
            }
            SoundNumberSourceOwner::SoundInput(siid) => {
                let input_data = self.sound_inputs.get_mut(&siid).unwrap();
                debug_assert!(!input_data.number_sources().contains(&id));
                input_data.number_sources_mut().push(id);
            }
        }

        let prev = self.number_sources.insert(id, data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_source(
        &mut self,
        source_id: SoundNumberSourceId,
        owner: SoundNumberSourceOwner,
    ) {
        // remove the number source from any number inputs that use it
        for ni_data in self.number_inputs.values_mut() {
            let (numbergraph, mapping) = ni_data.number_graph_and_mapping_mut();
            if mapping.target_graph_input(source_id).is_some() {
                mapping.remove_target(source_id, numbergraph);
            }
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

        // remove the number source
        self.number_sources.remove(&source_id).unwrap();
    }

    fn add_number_input(&mut self, data: SoundNumberInputData) {
        let id = data.id();

        let proc_data = self.sound_processors.get_mut(&data.owner()).unwrap();
        debug_assert!(!proc_data.number_inputs().contains(&id));
        proc_data.number_inputs_mut().push(id);

        let prev = self.number_inputs.insert(id, data);
        debug_assert!(prev.is_none());
    }

    fn remove_number_input(&mut self, id: SoundNumberInputId, owner: SoundProcessorId) {
        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        proc_data.number_inputs_mut().retain(|niid| *niid != id);

        self.number_inputs.remove(&id);
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
    fn get_revision(&self) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u64(self.sound_processors.get_revision());
        hasher.write_u64(self.sound_inputs.get_revision());
        hasher.write_u64(self.number_sources.get_revision());
        hasher.write_u64(self.number_inputs.get_revision());
        hasher.finish()
    }
}
