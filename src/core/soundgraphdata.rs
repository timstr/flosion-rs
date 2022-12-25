use std::sync::Arc;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceId, NumberSourceOwner},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{SoundProcessor, SoundProcessorId},
};

#[derive(Clone)]
pub(crate) struct SoundInputData {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    number_sources: Vec<NumberSourceId>,
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

    pub(super) fn id(&self) -> SoundInputId {
        self.id
    }

    pub(super) fn options(&self) -> InputOptions {
        self.options
    }

    pub(super) fn num_keys(&self) -> usize {
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

    pub(super) fn number_sources(&self) -> &Vec<NumberSourceId> {
        &self.number_sources
    }

    pub(super) fn number_sources_mut(&mut self) -> &mut Vec<NumberSourceId> {
        &mut self.number_sources
    }
}

#[derive(Clone)]
pub(crate) struct SoundProcessorData {
    id: SoundProcessorId,
    processor: Arc<dyn SoundProcessor>,
    sound_inputs: Vec<SoundInputId>,
    number_sources: Vec<NumberSourceId>,
    number_inputs: Vec<NumberInputId>,
}

impl SoundProcessorData {
    pub(super) fn new(processor: Arc<dyn SoundProcessor>) -> SoundProcessorData {
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

    pub(crate) fn number_sources(&self) -> &Vec<NumberSourceId> {
        &self.number_sources
    }

    pub(super) fn number_sources_mut(&mut self) -> &mut Vec<NumberSourceId> {
        &mut self.number_sources
    }

    pub(crate) fn number_inputs(&self) -> &Vec<NumberInputId> {
        &self.number_inputs
    }

    pub(super) fn number_inputs_mut(&mut self) -> &mut Vec<NumberInputId> {
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
pub(crate) struct NumberInputData {
    id: NumberInputId,
    target: Option<NumberSourceId>,
    owner: NumberInputOwner,
    default_value: f32,
}

impl NumberInputData {
    pub(super) fn new(
        id: NumberInputId,
        target: Option<NumberSourceId>,
        owner: NumberInputOwner,
        default_value: f32,
    ) -> Self {
        Self {
            id,
            target,
            owner,
            default_value,
        }
    }

    pub(super) fn id(&self) -> NumberInputId {
        self.id
    }

    pub(crate) fn target(&self) -> Option<NumberSourceId> {
        self.target
    }

    pub(super) fn set_target(&mut self, target: Option<NumberSourceId>) {
        self.target = target;
    }

    pub(crate) fn owner(&self) -> NumberInputOwner {
        self.owner
    }

    pub(super) fn default_value(&self) -> f32 {
        self.default_value
    }
}

#[derive(Clone)]
pub(crate) struct NumberSourceData {
    id: NumberSourceId,
    instance: Arc<dyn NumberSource>,
    owner: NumberSourceOwner,
    inputs: Vec<NumberInputId>,
}

impl NumberSourceData {
    pub(super) fn new(
        id: NumberSourceId,
        instance: Arc<dyn NumberSource>,
        owner: NumberSourceOwner,
    ) -> Self {
        Self {
            id,
            instance,
            owner,
            inputs: Vec::new(),
        }
    }

    pub(super) fn id(&self) -> NumberSourceId {
        self.id
    }

    pub(super) fn instance(&self) -> &dyn NumberSource {
        &*self.instance
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn NumberSource> {
        Arc::clone(&self.instance)
    }

    pub(crate) fn owner(&self) -> NumberSourceOwner {
        self.owner
    }

    pub(crate) fn inputs(&self) -> &Vec<NumberInputId> {
        &self.inputs
    }

    pub(super) fn inputs_mut(&mut self) -> &mut Vec<NumberInputId> {
        &mut self.inputs
    }
}
