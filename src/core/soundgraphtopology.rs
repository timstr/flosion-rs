use std::collections::{HashMap, HashSet};

use crate::core::soundnumbersource::SoundNumberSourceOwner;

use super::{
    graphobject::{GraphObjectHandle, ObjectId, SoundGraphId},
    soundgraphdata::{
        SoundInputData, SoundNumberInputData, SoundNumberSourceData, SoundProcessorData,
    },
    soundgraphedit::SoundGraphEdit,
    soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId,
    soundprocessor::SoundProcessorId,
};

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

    pub(crate) fn graph_object(&self, object_id: ObjectId) -> Option<GraphObjectHandle> {
        match object_id {
            ObjectId::Sound(spid) => self
                .sound_processors
                .get(&spid)
                .map(|p| p.instance_arc().as_graph_object()),
        }
    }

    pub(crate) fn graph_object_ids<'a>(&'a self) -> impl 'a + Iterator<Item = ObjectId> {
        let sound_objects = self.sound_processors.values().map(|x| x.id().into());
        sound_objects
    }

    pub(crate) fn graph_objects<'a>(&'a self) -> impl 'a + Iterator<Item = GraphObjectHandle> {
        let sound_objects = self
            .sound_processors
            .values()
            .map(|x| x.instance_arc().as_graph_object());
        sound_objects
    }

    pub(crate) fn make_edit(&mut self, edit: SoundGraphEdit) {
        match edit {
            SoundGraphEdit::AddSoundProcessor(data) => self.add_sound_processor(data),
            SoundGraphEdit::RemoveSoundProcessor(id) => self.remove_sound_processor(id),
            SoundGraphEdit::AddSoundInput(data) => self.add_sound_input(data),
            SoundGraphEdit::RemoveSoundInput(id, owner) => self.remove_sound_input(id, owner),
            SoundGraphEdit::AddSoundInputKey(siid, i) => self.add_sound_input_key(siid, i),
            SoundGraphEdit::RemoveSoundInputKey(siid, i) => self.remove_sound_input_key(siid, i),
            SoundGraphEdit::ConnectSoundInput(siid, spid) => self.connect_sound_input(siid, spid),
            SoundGraphEdit::DisconnectSoundInput(siid) => self.disconnect_sound_input(siid),
            SoundGraphEdit::AddNumberSource(data) => self.add_number_source(data),
            SoundGraphEdit::RemoveNumberSource(id, owner) => self.remove_number_source(id, owner),
            SoundGraphEdit::AddNumberInput(data) => self.add_number_input(data),
            SoundGraphEdit::RemoveNumberInput(id, owner) => self.remove_number_input(id, owner),
            SoundGraphEdit::ConnectNumberInput(niid, nsid) => self.connect_number_input(niid, nsid),
            SoundGraphEdit::DisconnectNumberInput(niid, nsid) => {
                self.disconnect_number_input(niid, nsid)
            }
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
        debug_assert!(self
            .number_inputs
            .values()
            .all(|ns| !ns.targets().contains(&source_id)));

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

    fn connect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
    ) {
        debug_assert!(self.number_sources.contains_key(&source_id));
        let input_data = self.number_inputs.get_mut(&input_id).unwrap();

        input_data.add_target(source_id);
    }

    fn disconnect_number_input(&mut self, input_id: SoundNumberInputId, nsid: SoundNumberSourceId) {
        let input_data = self.number_inputs.get_mut(&input_id).unwrap();
        input_data.remove_target(nsid);
    }
}

// TODO: precomputed dependencies?
// - does one processor depend on another?
//     - useful for validating number connection scope
// does one number input depend indirectly on a given source?
//     - useful for determining recompilation efficiently
// then again, recompilation can be determined once per edit
// and then applied to all affected number inputs.
// Also, recompilation could be deffered until the after the
// last edit in order to avoid wasted work. Affected number input
// nodes could simply be invalidated and then recompiled in a singel
// pass afterwards, which would have significant speedup for many
// edits in row
//
