use std::{hash::Hasher, sync::Arc};

use crate::core::{
    revision::revision::{Revision, RevisionNumber},
    uniqueid::UniqueId,
};

use super::{
    numbergraph::{NumberGraphInputId, NumberGraphOutputId},
    numberinput::NumberInputId,
    numbersource::{NumberSource, NumberSourceId},
};

#[derive(Clone)]
pub(crate) struct NumberSourceData {
    id: NumberSourceId,
    instance: Option<Arc<dyn NumberSource>>,
    inputs: Vec<NumberInputId>,
}

impl NumberSourceData {
    pub(crate) fn new(id: NumberSourceId, instance: Arc<dyn NumberSource>) -> NumberSourceData {
        NumberSourceData {
            id,
            instance: Some(instance),
            inputs: Vec::new(),
        }
    }

    pub(crate) fn new_empty(id: NumberSourceId) -> NumberSourceData {
        NumberSourceData {
            id,
            instance: None,
            inputs: Vec::new(),
        }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }

    pub(crate) fn instance(&self) -> &dyn NumberSource {
        self.instance.as_deref().unwrap()
    }

    pub(crate) fn instance_arc(&self) -> Arc<dyn NumberSource> {
        Arc::clone(self.instance.as_ref().unwrap())
    }

    pub(crate) fn set_instance(&mut self, instance: Arc<dyn NumberSource>) {
        assert!(self.instance.is_none());
        self.instance = Some(instance);
    }

    pub fn number_inputs(&self) -> &[NumberInputId] {
        &self.inputs
    }

    pub fn number_inputs_mut(&mut self) -> &mut Vec<NumberInputId> {
        &mut self.inputs
    }
}

impl Revision for NumberSourceData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hasher.write_usize(self.inputs.len());
        for niid in &self.inputs {
            hasher.write_usize(niid.value());
        }
        RevisionNumber::new(hasher.finish())
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum NumberTarget {
    // TODO: Empty
    Source(NumberSourceId),
    GraphInput(NumberGraphInputId),
}

impl From<NumberSourceId> for NumberTarget {
    fn from(value: NumberSourceId) -> Self {
        NumberTarget::Source(value)
    }
}

impl From<NumberGraphInputId> for NumberTarget {
    fn from(value: NumberGraphInputId) -> Self {
        NumberTarget::GraphInput(value)
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum NumberDestination {
    Input(NumberInputId),
    GraphOutput(NumberGraphOutputId),
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

fn hash_optional_target(target: Option<NumberTarget>, hasher: &mut seahash::SeaHasher) {
    match target {
        Some(NumberTarget::GraphInput(giid)) => {
            hasher.write_u8(1);
            hasher.write_usize(giid.value());
        }
        Some(NumberTarget::Source(nsid)) => {
            hasher.write_u8(2);
            hasher.write_usize(nsid.value());
        }
        None => {
            hasher.write_u8(3);
        }
    }
}

impl Revision for NumberInputData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hash_optional_target(self.target, &mut hasher);
        hasher.write_usize(self.owner.value());
        hasher.write_u32(self.default_value.to_bits());
        RevisionNumber::new(hasher.finish())
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

impl Revision for NumberGraphOutputData {
    fn get_revision(&self) -> RevisionNumber {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.id.value());
        hash_optional_target(self.target, &mut hasher);
        hasher.write_u32(self.default_value.to_bits());
        RevisionNumber::new(hasher.finish())
    }
}
