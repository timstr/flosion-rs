use std::sync::Arc;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceId, NumberSourceOwner},
    soundgraphdescription::{
        NumberInputDescription, NumberSourceDescription, SoundInputDescription,
        SoundProcessorDescription,
    },
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{SoundProcessor, SoundProcessorId},
};

pub(crate) struct EngineSoundInputData {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    number_sources: Vec<NumberSourceId>,
}

impl EngineSoundInputData {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
        num_keys: usize,
        owner: SoundProcessorId,
    ) -> EngineSoundInputData {
        EngineSoundInputData {
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

    pub(super) fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub(super) fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    pub(super) fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    pub(super) fn number_sources(&self) -> &Vec<NumberSourceId> {
        &self.number_sources
    }

    pub(super) fn number_sources_mut(&mut self) -> &mut Vec<NumberSourceId> {
        &mut self.number_sources
    }

    pub(super) fn describe(&self) -> SoundInputDescription {
        SoundInputDescription::new(
            self.id,
            self.options,
            self.num_keys,
            self.target,
            self.owner,
            self.number_sources.clone(),
        )
    }
}

pub(crate) struct EngineSoundProcessorData {
    id: SoundProcessorId,
    processor: Option<Arc<dyn SoundProcessor>>,
    sound_inputs: Vec<SoundInputId>,
    number_sources: Vec<NumberSourceId>,
    number_inputs: Vec<NumberInputId>,
}

impl EngineSoundProcessorData {
    pub(super) fn new_without_processor(id: SoundProcessorId) -> EngineSoundProcessorData {
        EngineSoundProcessorData {
            id,
            processor: None,
            sound_inputs: Vec::new(),
            number_sources: Vec::new(),
            number_inputs: Vec::new(),
        }
    }

    pub(super) fn set_processor(&mut self, processor: Arc<dyn SoundProcessor>) {
        debug_assert!(self.processor.is_none());
        self.processor = Some(processor);
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
        &**self.processor.as_ref().unwrap()
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn SoundProcessor> {
        Arc::clone(self.processor.as_ref().unwrap())
    }

    pub(super) fn describe(&self) -> SoundProcessorDescription {
        SoundProcessorDescription::new(
            self.id,
            self.instance().is_static(),
            self.sound_inputs.clone(),
            self.number_sources.clone(),
            self.number_inputs.clone(),
        )
    }
}

pub(crate) struct EngineNumberInputData {
    id: NumberInputId,
    target: Option<NumberSourceId>,
    owner: NumberInputOwner,
    default_value: f32,
}

impl EngineNumberInputData {
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

    pub(super) fn target(&self) -> Option<NumberSourceId> {
        self.target
    }

    pub(super) fn set_target(&mut self, target: Option<NumberSourceId>) {
        self.target = target;
    }

    pub(super) fn owner(&self) -> NumberInputOwner {
        self.owner
    }

    pub(super) fn describe(&self) -> NumberInputDescription {
        NumberInputDescription::new(self.id, self.target, self.owner)
    }

    pub(super) fn default_value(&self) -> f32 {
        self.default_value
    }
}

pub(crate) struct EngineNumberSourceData {
    id: NumberSourceId,
    instance: Option<Arc<dyn NumberSource>>,
    owner: NumberSourceOwner,
    inputs: Vec<NumberInputId>,
}

impl EngineNumberSourceData {
    pub(super) fn new(
        id: NumberSourceId,
        wrapper: Option<Arc<dyn NumberSource>>,
        owner: NumberSourceOwner,
    ) -> Self {
        Self {
            id,
            instance: wrapper,
            owner,
            inputs: Vec::new(),
        }
    }

    pub(super) fn set_source(&mut self, source: Arc<dyn NumberSource>) {
        debug_assert!(self.instance.is_none());
        self.instance = Some(source);
    }

    pub(super) fn id(&self) -> NumberSourceId {
        self.id
    }

    pub(super) fn instance(&self) -> &dyn NumberSource {
        &**self.instance.as_ref().unwrap()
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn NumberSource> {
        Arc::clone(self.instance.as_ref().unwrap())
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

    pub(super) fn describe(&self) -> NumberSourceDescription {
        NumberSourceDescription::new(self.id, self.inputs.clone(), self.owner)
    }
}
