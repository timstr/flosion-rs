use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
    sync::Arc,
};

use crate::core::{
    number::{
        numbergraph::{NumberGraph, NumberGraphInputId},
        numbergraphtopology::NumberGraphTopology,
    },
    revision::revision::{Revision, RevisionNumber},
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
    time_number_source: SoundNumberSourceId,
}

impl SoundInputData {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
        num_keys: usize,
        owner: SoundProcessorId,
        time_number_source: SoundNumberSourceId,
    ) -> SoundInputData {
        SoundInputData {
            id,
            options,
            num_keys,
            target: None,
            owner,
            number_sources: Vec::new(),
            time_number_source,
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

    pub(crate) fn time_number_source(&self) -> SoundNumberSourceId {
        self.time_number_source
    }
}

impl Revision for SoundInputData {
    fn get_revision(&self) -> RevisionNumber {
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
        RevisionNumber::new(hasher.finish())
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
    fn get_revision(&self) -> RevisionNumber {
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
        RevisionNumber::new(hasher.finish())
    }
}

#[derive(Clone)]
pub(crate) struct SoundNumberInputTargetMapping {
    mapping: HashMap<NumberGraphInputId, SoundNumberSourceId>,
}

impl SoundNumberInputTargetMapping {
    fn new() -> SoundNumberInputTargetMapping {
        SoundNumberInputTargetMapping {
            mapping: HashMap::new(),
        }
    }

    pub(crate) fn graph_input_target(&self, id: NumberGraphInputId) -> Option<SoundNumberSourceId> {
        self.mapping.get(&id).cloned()
    }

    pub(crate) fn target_graph_input(&self, id: SoundNumberSourceId) -> Option<NumberGraphInputId> {
        for (giid, nsid) in &self.mapping {
            if *nsid == id {
                return Some(*giid);
            }
        }
        None
    }

    // NOTE: passing NumberGraph separately here might seem a bit awkward from the perspective of the
    // SoundNumberInputData that owns this and the number graph, but it makes the two separable.
    // This is useful for making LexicalLayout more reusable accross different types of number graphs
    pub(crate) fn add_target(
        &mut self,
        source_id: SoundNumberSourceId,
        numbergraph: &mut NumberGraph,
    ) -> NumberGraphInputId {
        debug_assert!(self.check_invariants(numbergraph.topology()));
        if let Some(giid) = self.target_graph_input(source_id) {
            return giid;
        }
        let giid = numbergraph.add_graph_input();
        let prev = self.mapping.insert(giid, source_id);
        debug_assert_eq!(prev, None);
        debug_assert!(self.check_invariants(numbergraph.topology()));
        giid
    }

    pub(crate) fn remove_target(
        &mut self,
        source_id: SoundNumberSourceId,
        numbergraph: &mut NumberGraph,
    ) {
        debug_assert!(self.check_invariants(numbergraph.topology()));
        let giid = self.target_graph_input(source_id).unwrap();
        numbergraph.remove_graph_input(giid).unwrap();
        let prev = self.mapping.remove(&giid);
        debug_assert!(prev.is_some());
        debug_assert!(self.check_invariants(numbergraph.topology()));
    }

    fn check_invariants(&self, topology: &NumberGraphTopology) -> bool {
        let mapped_graph_inputs: HashSet<NumberGraphInputId> =
            self.mapping.keys().cloned().collect();
        let actual_graph_inputs: HashSet<NumberGraphInputId> =
            topology.graph_inputs().iter().cloned().collect();
        if mapped_graph_inputs != actual_graph_inputs {
            println!("Number graph inputs were modified without number input mapping");
            false
        } else {
            true
        }
    }

    pub(crate) fn items(&self) -> &HashMap<NumberGraphInputId, SoundNumberSourceId> {
        &self.mapping
    }
}

#[derive(Clone)]
pub struct SoundNumberInputScope {
    processor_state_available: bool,
    available_local_sources: Vec<SoundNumberSourceId>,
}

impl SoundNumberInputScope {
    pub fn without_processor_state() -> SoundNumberInputScope {
        SoundNumberInputScope {
            processor_state_available: false,
            available_local_sources: Vec::new(),
        }
    }

    pub fn with_processor_state() -> SoundNumberInputScope {
        SoundNumberInputScope {
            processor_state_available: true,
            available_local_sources: Vec::new(),
        }
    }

    pub fn add_local(mut self, id: SoundNumberSourceId) -> SoundNumberInputScope {
        self.available_local_sources.push(id);
        self
    }

    pub(crate) fn processor_state_available(&self) -> bool {
        self.processor_state_available
    }

    pub(crate) fn available_local_sources(&self) -> &[SoundNumberSourceId] {
        &self.available_local_sources
    }
}

impl Revision for SoundNumberInputScope {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u8(if self.processor_state_available { 1 } else { 0 });
        hasher.write_usize(self.available_local_sources.len());
        for nsid in &self.available_local_sources {
            hasher.write_usize(nsid.value());
        }
        RevisionNumber::new(hasher.finish())
    }
}

#[derive(Clone)]
pub struct SoundNumberInputData {
    id: SoundNumberInputId,
    target_mapping: SoundNumberInputTargetMapping,
    number_graph: NumberGraph,
    owner: SoundProcessorId,
    scope: SoundNumberInputScope,
}

impl SoundNumberInputData {
    pub(crate) fn new(
        id: SoundNumberInputId,
        owner: SoundProcessorId,
        default_value: f32,
        scope: SoundNumberInputScope,
    ) -> Self {
        let mut number_graph = NumberGraph::new();

        // HACK: assuming 1 output for now
        number_graph.add_graph_output(default_value);

        Self {
            id,
            target_mapping: SoundNumberInputTargetMapping::new(),
            number_graph,
            owner,
            scope,
        }
    }

    pub(crate) fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub(crate) fn target_mapping(&self) -> &SoundNumberInputTargetMapping {
        debug_assert!(self
            .target_mapping
            .check_invariants(self.number_graph.topology()));
        &self.target_mapping
    }

    pub(crate) fn number_graph(&self) -> &NumberGraph {
        &self.number_graph
    }

    pub(crate) fn number_graph_mut(&mut self) -> &mut NumberGraph {
        &mut self.number_graph
    }

    pub(crate) fn number_graph_and_mapping_mut(
        &mut self,
    ) -> (&mut NumberGraph, &mut SoundNumberInputTargetMapping) {
        (&mut self.number_graph, &mut self.target_mapping)
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    pub(crate) fn scope(&self) -> &SoundNumberInputScope {
        &self.scope
    }
}

impl Revision for SoundNumberInputData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        let items_hash: u64 = 0;
        for (giid, nsid) in self.target_mapping.items() {
            hasher.write_usize(giid.value());
            hasher.write_usize(nsid.value());
        }
        hasher.write_u64(items_hash);
        hasher.write_u64(self.number_graph.topology().get_revision().value());
        hasher.write_usize(self.owner.value());
        hasher.write_u64(self.scope.get_revision().value());
        RevisionNumber::new(hasher.finish())
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

    pub(crate) fn id(&self) -> SoundNumberSourceId {
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
    fn get_revision(&self) -> RevisionNumber {
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
        RevisionNumber::new(hasher.finish())
    }
}
