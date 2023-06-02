use std::sync::Arc;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numbersource::{NumberSource, NumberSourceId},
};

#[derive(Clone)]
pub(crate) struct NumberSourceInstanceData {
    id: NumberSourceId,
    instance: Arc<dyn NumberSource>,
    inputs: Vec<NumberInputId>,
}

impl NumberSourceInstanceData {
    pub(crate) fn new(
        id: NumberSourceId,
        instance: Arc<dyn NumberSource>,
    ) -> NumberSourceInstanceData {
        NumberSourceInstanceData {
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

#[derive(Clone)]
pub(crate) enum NumberSourceData {
    Instance(NumberSourceInstanceData),
    GraphInput(NumberSourceId),
}

impl NumberSourceData {
    pub(crate) fn new_instance(
        id: NumberSourceId,
        instance: Arc<dyn NumberSource>,
    ) -> NumberSourceData {
        NumberSourceData::Instance(NumberSourceInstanceData::new(id, instance))
    }

    pub(crate) fn new_graph_input(id: NumberSourceId) -> NumberSourceData {
        NumberSourceData::GraphInput(id)
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        match self {
            NumberSourceData::Instance(data) => data.id(),
            NumberSourceData::GraphInput(id) => *id,
        }
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
    pub(crate) fn new(
        id: NumberInputId,
        owner: NumberInputOwner,
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

    pub fn target(&self) -> Option<NumberSourceId> {
        self.target
    }

    pub fn owner(&self) -> NumberInputOwner {
        self.owner
    }

    pub fn default_value(&self) -> f32 {
        self.default_value
    }
}
