use std::sync::Arc;

use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numberinput::NumberInputId,
    numbersource::{NumberSource, NumberSourceId},
};

#[derive(Clone)]
pub(crate) struct NumberSourceData {
    id: NumberSourceId,
    instance: Arc<dyn NumberSource>,
    inputs: Vec<NumberInputId>,
}

impl NumberSourceData {
    pub(crate) fn new(id: NumberSourceId, instance: Arc<dyn NumberSource>) -> NumberSourceData {
        NumberSourceData {
            id,
            instance,
            inputs: Vec::new(),
        }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn NumberSource {
        &*self.instance
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn NumberSource> {
        Arc::clone(&self.instance)
    }

    pub fn number_inputs(&self) -> &[NumberInputId] {
        &self.inputs
    }

    pub fn number_inputs_mut(&mut self) -> &mut Vec<NumberInputId> {
        &mut self.inputs
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum NumberTarget {
    Source(NumberSourceId),
    GraphInput(NumberGraphInputId),
}

#[derive(Clone)]
pub(crate) struct NumberInputData {
    id: NumberInputId,
    target: Option<NumberTarget>,
    owner: NumberSourceId,
    default_value: f32,
}

impl NumberInputData {
    pub(crate) fn new(
        id: NumberInputId,
        owner: NumberSourceId,
        default_value: f32,
    ) -> NumberInputData {
        NumberInputData {
            id,
            target: None,
            owner,
            default_value,
        }
    }

    pub fn id(&self) -> NumberInputId {
        self.id
    }

    pub fn target(&self) -> Option<NumberTarget> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<NumberTarget>) {
        self.target = target;
    }

    pub fn owner(&self) -> NumberSourceId {
        self.owner
    }

    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}

#[derive(Clone)]
pub(crate) struct NumberGraphOutputData {
    id: NumberGraphOutputId,
    target: Option<NumberTarget>,
    default_value: f32,
}

impl NumberGraphOutputData {
    pub(crate) fn new(id: NumberGraphOutputId, default_value: f32) -> NumberGraphOutputData {
        NumberGraphOutputData {
            id,
            target: None,
            default_value,
        }
    }

    pub fn id(&self) -> NumberGraphOutputId {
        self.id
    }

    pub fn target(&self) -> Option<NumberTarget> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<NumberTarget>) {
        self.target = target;
    }

    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}
