use std::sync::Arc;

use super::{
    numbergraph::NumberGraph,
    numbersource::NumberSourceId,
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

    pub(super) fn id(&self) -> SoundProcessorId {
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

    pub(super) fn instance(&self) -> &dyn SoundProcessor {
        &*self.processor
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn SoundProcessor> {
        Arc::clone(&self.processor)
    }
}

#[derive(Clone)]
pub(crate) struct SoundNumberInputData {
    id: SoundNumberInputId,
    targets: Vec<SoundNumberSourceId>,
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
            targets: Vec::new(),
            number_graph,
            owner,
        }
    }

    pub(super) fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub(crate) fn targets(&self) -> &[SoundNumberSourceId] {
        &self.targets
    }

    pub(super) fn add_target(&mut self, target: SoundNumberSourceId) {
        debug_assert_eq!(self.targets.iter().filter(|t| **t == target).count(), 0);
        self.targets.push(target);
        self.number_graph.add_graph_input();
    }

    pub(super) fn remove_target(&mut self, target: SoundNumberSourceId) {
        // TODO: consider something nicer than assuming that number graph
        // inputs and sound number source targets always match up 1:1
        debug_assert_eq!(self.targets.iter().filter(|t| **t == target).count(), 1);
        let i = self.targets.iter().position(|t| *t == target).unwrap();
        self.targets.remove(i);
        let niid = self.number_graph.topology().graph_inputs()[i];
        self.number_graph.remove_graph_input(niid);
    }

    pub(crate) fn number_graph(&self) -> &NumberGraph {
        &self.number_graph
    }

    pub(crate) fn input_mapping<'a>(
        &'a self,
    ) -> impl 'a + Iterator<Item = (SoundNumberSourceId, NumberSourceId)> {
        let number_topo = self.number_graph.topology();
        debug_assert_eq!(self.targets.len(), number_topo.graph_inputs().len());
        self.targets
            .iter()
            .cloned()
            .zip(number_topo.graph_inputs().iter().cloned())
    }

    pub(crate) fn owner(&self) -> SoundProcessorId {
        self.owner
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
