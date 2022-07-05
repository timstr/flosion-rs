use std::sync::Arc;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceId, NumberSourceOwner},
    soundinput::{SoundInputId, SoundInputWrapper},
    soundprocessor::{SoundProcessorData, SoundProcessorId, SoundProcessorWrapper},
};

pub struct EngineSoundInputData {
    input: Arc<dyn SoundInputWrapper>,
    target: Option<SoundProcessorId>,
    owner: SoundProcessorId,
    number_sources: Vec<NumberSourceId>,
}

impl EngineSoundInputData {
    pub fn new(input: Arc<dyn SoundInputWrapper>, owner: SoundProcessorId) -> EngineSoundInputData {
        EngineSoundInputData {
            input,
            target: None,
            owner,
            number_sources: Vec::new(),
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.input.id()
    }

    pub fn target(&self) -> Option<SoundProcessorId> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<SoundProcessorId>) {
        self.target = target;
    }

    pub fn input(&self) -> &dyn SoundInputWrapper {
        &*self.input
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
}

impl Clone for EngineSoundInputData {
    fn clone(&self) -> Self {
        Self {
            input: Arc::clone(&self.input),
            target: self.target.clone(),
            owner: self.owner.clone(),
            number_sources: self.number_sources.clone(),
        }
    }
}

pub struct EngineSoundProcessorData {
    id: SoundProcessorId,
    wrapper: Arc<dyn SoundProcessorWrapper>,
    inputs: Vec<SoundInputId>, // TODO: rename to sound_inputs
    number_sources: Vec<NumberSourceId>,
    number_inputs: Vec<NumberInputId>,
}

impl EngineSoundProcessorData {
    pub fn new(
        wrapper: Arc<dyn SoundProcessorWrapper>,
        id: SoundProcessorId,
    ) -> EngineSoundProcessorData {
        EngineSoundProcessorData {
            id,
            wrapper,
            inputs: Vec::new(),
            number_sources: Vec::new(),
            number_inputs: Vec::new(),
        }
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id
    }

    // TODO: rename to sound_inputs
    pub fn inputs(&self) -> &Vec<SoundInputId> {
        &self.inputs
    }

    // TODO: rename to sound_inputs_mut
    pub fn inputs_mut(&mut self) -> &mut Vec<SoundInputId> {
        &mut self.inputs
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

    pub fn wrapper(&self) -> &dyn SoundProcessorWrapper {
        &*self.wrapper
    }
}

impl Clone for EngineSoundProcessorData {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            wrapper: Arc::clone(&self.wrapper),
            inputs: self.inputs.clone(),
            number_sources: self.number_sources.clone(),
            number_inputs: self.number_inputs.clone(),
        }
    }
}

#[derive(Clone)]
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
}

pub struct EngineNumberSourceData {
    id: NumberSourceId,
    wrapper: Arc<dyn NumberSource>,
    owner: NumberSourceOwner,
    inputs: Vec<NumberInputId>,
}

impl EngineNumberSourceData {
    pub fn new(
        id: NumberSourceId,
        wrapper: Arc<dyn NumberSource>,
        owner: NumberSourceOwner,
        inputs: Vec<NumberInputId>,
    ) -> Self {
        Self {
            id,
            wrapper,
            owner,
            inputs,
        }
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }

    pub fn instance(&self) -> &dyn NumberSource {
        &*self.wrapper
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
}

impl Clone for EngineNumberSourceData {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            wrapper: Arc::clone(&self.wrapper),
            owner: self.owner.clone(),
            inputs: self.inputs.clone(),
        }
    }
}
