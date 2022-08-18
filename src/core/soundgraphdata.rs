use std::sync::Arc;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceId, NumberSourceOwner},
    soundgraphdescription::{
        NumberInputDescription, NumberSourceDescription, SoundInputDescription,
        SoundProcessorDescription,
    },
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{SoundProcessorId, SoundProcessorWrapper},
};

pub struct EngineSoundInputData {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    number_sources: Vec<NumberSourceId>,
}

impl EngineSoundInputData {
    pub fn new(
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

    pub fn id(&self) -> SoundInputId {
        self.id
    }

    pub fn options(&self) -> InputOptions {
        self.options
    }

    pub fn num_keys(&self) -> usize {
        self.num_keys
    }

    pub fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    pub fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    pub fn number_sources(&self) -> &Vec<NumberSourceId> {
        &self.number_sources
    }

    pub fn number_sources_mut(&mut self) -> &mut Vec<NumberSourceId> {
        &mut self.number_sources
    }

    pub fn describe(&self) -> SoundInputDescription {
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

pub struct EngineSoundProcessorData {
    id: SoundProcessorId,
    processor: Option<Arc<dyn SoundProcessorWrapper>>,
    sound_inputs: Vec<SoundInputId>,
    number_sources: Vec<NumberSourceId>,
    number_inputs: Vec<NumberInputId>,
}

impl EngineSoundProcessorData {
    pub fn new_without_processor(id: SoundProcessorId) -> EngineSoundProcessorData {
        EngineSoundProcessorData {
            id,
            processor: None,
            sound_inputs: Vec::new(),
            number_sources: Vec::new(),
            number_inputs: Vec::new(),
        }
    }

    pub fn set_processor(&mut self, processor: Arc<dyn SoundProcessorWrapper>) {
        debug_assert!(self.processor.is_none());
        self.processor = Some(processor);
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    pub fn sound_inputs(&self) -> &Vec<SoundInputId> {
        &self.sound_inputs
    }

    pub fn sound_inputs_mut(&mut self) -> &mut Vec<SoundInputId> {
        &mut self.sound_inputs
    }

    pub fn number_sources(&self) -> &Vec<NumberSourceId> {
        &self.number_sources
    }

    pub fn number_sources_mut(&mut self) -> &mut Vec<NumberSourceId> {
        &mut self.number_sources
    }

    pub fn number_inputs(&self) -> &Vec<NumberInputId> {
        &self.number_inputs
    }

    pub fn number_inputs_mut(&mut self) -> &mut Vec<NumberInputId> {
        &mut self.number_inputs
    }

    pub fn processor(&self) -> &dyn SoundProcessorWrapper {
        &**self.processor.as_ref().unwrap()
    }

    pub fn processor_arc(&self) -> Arc<dyn SoundProcessorWrapper> {
        Arc::clone(self.processor.as_ref().unwrap())
    }

    pub fn describe(&self) -> SoundProcessorDescription {
        SoundProcessorDescription::new(
            self.id,
            self.processor().is_static(),
            self.sound_inputs.clone(),
            self.number_sources.clone(),
            self.number_inputs.clone(),
        )
    }
}

pub struct EngineNumberInputData {
    id: NumberInputId,
    target: Option<NumberSourceId>,
    owner: NumberInputOwner,
}

impl EngineNumberInputData {
    pub fn new(id: NumberInputId, target: Option<NumberSourceId>, owner: NumberInputOwner) -> Self {
        Self { id, target, owner }
    }

    pub fn id(&self) -> NumberInputId {
        self.id
    }

    pub fn target(&self) -> Option<NumberSourceId> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<NumberSourceId>) {
        self.target = target;
    }

    pub fn owner(&self) -> NumberInputOwner {
        self.owner
    }

    pub fn describe(&self) -> NumberInputDescription {
        NumberInputDescription::new(self.id, self.target, self.owner)
    }
}

pub struct EngineNumberSourceData {
    id: NumberSourceId,
    instance: Option<Arc<dyn NumberSource>>,
    owner: NumberSourceOwner,
    inputs: Vec<NumberInputId>,
}

impl EngineNumberSourceData {
    pub fn new(
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

    pub fn set_source(&mut self, source: Arc<dyn NumberSource>) {
        debug_assert!(self.instance.is_none());
        self.instance = Some(source);
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }

    pub fn instance(&self) -> &dyn NumberSource {
        &**self.instance.as_ref().unwrap()
    }

    pub fn instance_arc(&self) -> Arc<dyn NumberSource> {
        Arc::clone(self.instance.as_ref().unwrap())
    }

    pub fn owner(&self) -> NumberSourceOwner {
        self.owner
    }

    pub fn inputs(&self) -> &Vec<NumberInputId> {
        &self.inputs
    }

    pub fn inputs_mut(&mut self) -> &mut Vec<NumberInputId> {
        &mut self.inputs
    }

    pub fn describe(&self) -> NumberSourceDescription {
        NumberSourceDescription::new(self.id, self.inputs.clone(), self.owner)
    }
}
