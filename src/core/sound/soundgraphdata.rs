use std::{collections::HashMap, hash::Hasher, sync::Arc};

use crate::core::{
    number::numbergraph::{NumberGraph, NumberGraphInputId},
    revision::Revision,
    uniqueid::UniqueId,
};

use super::{
    soundinput::{InputOptions, SoundInputId},
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::{SoundNumberSource, SoundNumberSourceId, SoundNumberSourceOwner},
    soundprocessor::{SoundProcessor, SoundProcessorId},
};

#[derive(Clone)]
pub(crate) struct SoundInputData {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    number_sources: Vec<SoundNumberSourceId>,
}

impl SoundInputData {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
        num_keys: usize,
        owner: SoundProcessorId,
    ) -> SoundInputData {
        SoundInputData {
            id,
            options,
            num_keys,
            target: None,
            owner,
            number_sources: Vec::new(),
        }
    }

    pub(crate) fn id(&self) -> SoundInputId {
        self.id
    }

    pub(crate) fn options(&self) -> InputOptions {
        self.options
    }

    pub(crate) fn num_keys(&self) -> usize {
        self.num_keys
    }

    pub(super) fn set_num_keys(&mut self, n: usize) {
        self.num_keys = n;
    }

    pub(crate) fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub(super) fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    pub(crate) fn number_sources(&self) -> &Vec<SoundNumberSourceId> {
        &self.number_sources
    }

    pub(crate) fn number_sources_mut(&mut self) -> &mut Vec<SoundNumberSourceId> {
        &mut self.number_sources
    }
}

impl Revision for SoundInputData {
    fn get_revision(&self) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hasher.write_u8(match &self.options {
            InputOptions::Synchronous => 0x1,
            InputOptions::NonSynchronous => 0x2,
        });
        hasher.write_usize(self.num_keys);
        hasher.write_usize(match &self.target {
            Some(id) => id.value(),
            None => usize::MAX,
        });
        hasher.write_usize(self.owner.value());
        hasher.write_usize(self.number_sources.len());
        for nsid in &self.number_sources {
            hasher.write_usize(nsid.value());
        }
        hasher.finish()
    }
}

#[derive(Clone)]
pub(crate) struct SoundProcessorData {
    id: SoundProcessorId,
    processor: Arc<dyn SoundProcessor>,
    sound_inputs: Vec<SoundInputId>,
    number_sources: Vec<SoundNumberSourceId>,
    number_inputs: Vec<SoundNumberInputId>,
}

impl SoundProcessorData {
    pub(crate) fn new(processor: Arc<dyn SoundProcessor>) -> SoundProcessorData {
        SoundProcessorData {
            id: processor.id(),
            processor,
            sound_inputs: Vec::new(),
            number_sources: Vec::new(),
            number_inputs: Vec::new(),
        }
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub(crate) fn sound_inputs(&self) -> &Vec<SoundInputId> {
        &self.sound_inputs
    }

    pub(super) fn sound_inputs_mut(&mut self) -> &mut Vec<SoundInputId> {
        &mut self.sound_inputs
    }

    pub(crate) fn number_sources(&self) -> &Vec<SoundNumberSourceId> {
        &self.number_sources
    }

    pub(super) fn number_sources_mut(&mut self) -> &mut Vec<SoundNumberSourceId> {
        &mut self.number_sources
    }

    pub(crate) fn number_inputs(&self) -> &Vec<SoundNumberInputId> {
        &self.number_inputs
    }

    pub(super) fn number_inputs_mut(&mut self) -> &mut Vec<SoundNumberInputId> {
        &mut self.number_inputs
    }

    pub(crate) fn instance(&self) -> &dyn SoundProcessor {
        &*self.processor
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn SoundProcessor> {
        Arc::clone(&self.processor)
    }
}

impl Revision for SoundProcessorData {
    fn get_revision(&self) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hasher.write_u8(if self.processor.is_static() { 1 } else { 2 });
        // Do not hash processor instance
        hasher.write_usize(self.sound_inputs.len());
        for siid in &self.sound_inputs {
            hasher.write_usize(siid.value());
        }
        hasher.write_usize(self.number_sources.len());
        for nsid in &self.number_sources {
            hasher.write_usize(nsid.value());
        }
        hasher.write_usize(self.number_inputs.len());
        for niid in &self.number_inputs {
            hasher.write_usize(niid.value());
        }
        hasher.finish()
    }
}

#[derive(Clone)]
pub struct SoundNumberInputData {
    id: SoundNumberInputId,
    target_mapping: HashMap<NumberGraphInputId, SoundNumberSourceId>,
    number_graph: NumberGraph,
    owner: SoundProcessorId,
}

impl SoundNumberInputData {
    pub(crate) fn new(id: SoundNumberInputId, owner: SoundProcessorId, default_value: f32) -> Self {
        let mut number_graph = NumberGraph::new();

        // HACK: assuming 1 output for now
        number_graph.add_graph_output(default_value);

        Self {
            id,
            target_mapping: HashMap::new(),
            number_graph,
            owner,
        }
    }

    pub(crate) fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub(crate) fn target_mapping(&self) -> &HashMap<NumberGraphInputId, SoundNumberSourceId> {
        &self.target_mapping
    }

    // pub(crate) fn target_mapping_mut(
    //     &mut self,
    // ) -> &mut HashMap<NumberGraphInputId, SoundNumberSourceId> {
    //     &mut self.target_mapping
    // }

    pub(crate) fn number_graph(&self) -> &NumberGraph {
        &self.number_graph
    }

    pub(crate) fn number_graph_mut(&mut self) -> &mut NumberGraph {
        &mut self.number_graph
    }

    pub(crate) fn graph_input_target(&self, id: NumberGraphInputId) -> Option<SoundNumberSourceId> {
        self.target_mapping.get(&id).cloned()
    }

    pub(crate) fn target_graph_input(&self, id: SoundNumberSourceId) -> Option<NumberGraphInputId> {
        for (giid, nsid) in &self.target_mapping {
            if *nsid == id {
                return Some(*giid);
            }
        }
        None
    }

    pub(crate) fn add_target(&mut self, source_id: SoundNumberSourceId) -> NumberGraphInputId {
        if let Some(giid) = self.target_graph_input(source_id) {
            return giid;
        }
        let giid = self.number_graph.add_graph_input();
        let prev = self.target_mapping.insert(giid, source_id);
        debug_assert_eq!(prev, None);
        giid
    }

    pub(crate) fn remove_target(&mut self, source_id: SoundNumberSourceId) {
        let giid = self.target_graph_input(source_id).unwrap();
        self.number_graph.remove_graph_input(giid).unwrap();
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
    }
}

impl Revision for SoundNumberInputData {
    fn get_revision(&self) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        let items_hash: u64 = 0;
        for (giid, nsid) in &self.target_mapping {
            hasher.write_usize(giid.value());
            hasher.write_usize(nsid.value());
        }
        hasher.write_u64(items_hash);
        hasher.write_u64(self.number_graph.topology().get_revision());
        hasher.write_usize(self.owner.value());
        hasher.finish()
    }
}

#[derive(Clone)]
pub(crate) struct SoundNumberSourceData {
    id: SoundNumberSourceId,
    instance: Arc<dyn SoundNumberSource>,
    owner: SoundNumberSourceOwner,
}

impl SoundNumberSourceData {
    pub(crate) fn new(
        id: SoundNumberSourceId,
        instance: Arc<dyn SoundNumberSource>,
        owner: SoundNumberSourceOwner,
    ) -> Self {
        Self {
            id,
            instance,
            owner,
        }
    }

    pub(super) fn id(&self) -> SoundNumberSourceId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn SoundNumberSource {
        &*self.instance
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn SoundNumberSource> {
        Arc::clone(&self.instance)
    }

    pub(crate) fn owner(&self) -> SoundNumberSourceOwner {
        self.owner
    }
}

impl Revision for SoundNumberSourceData {
    fn get_revision(&self) -> u64 {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        // Do not hash instance
        match &self.owner {
            SoundNumberSourceOwner::SoundProcessor(spid) => {
                hasher.write_u8(1);
                hasher.write_usize(spid.value());
            }
            SoundNumberSourceOwner::SoundInput(siid) => {
                hasher.write_u8(2);
                hasher.write_usize(siid.value());
            }
        }
        hasher.finish()
    }
}
