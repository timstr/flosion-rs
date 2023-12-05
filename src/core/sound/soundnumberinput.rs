use crate::core::{
    engine::{nodegen::NodeGen, soundnumberinputnode::SoundNumberInputNode},
    uniqueid::UniqueId,
};

use super::{soundgraphdata::SoundNumberInputScope, soundprocessor::SoundProcessorId};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundNumberInputId(usize);

impl SoundNumberInputId {
    pub(crate) fn new(id: usize) -> SoundNumberInputId {
        SoundNumberInputId(id)
    }
}

impl Default for SoundNumberInputId {
    fn default() -> Self {
        SoundNumberInputId(1)
    }
}

impl UniqueId for SoundNumberInputId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> Self {
        SoundNumberInputId(self.0 + 1)
    }
}

pub struct SoundNumberInputHandle {
    id: SoundNumberInputId,
    owner: SoundProcessorId,

    #[cfg(debug_assertions)]
    scope: SoundNumberInputScope,
}

impl SoundNumberInputHandle {
    // TODO: why are these new() functions pub?

    #[cfg(not(debug_assertions))]
    pub fn new(id: SoundNumberInputId, owner: SoundProcessorId) -> SoundNumberInputHandle {
        SoundNumberInputHandle { id, owner }
    }

    #[cfg(debug_assertions)]
    pub fn new(
        id: SoundNumberInputId,
        owner: SoundProcessorId,
        scope: SoundNumberInputScope,
    ) -> SoundNumberInputHandle {
        SoundNumberInputHandle { id, owner, scope }
    }

    pub fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub fn owner(&self) -> SoundProcessorId {
        self.owner
    }

    #[cfg(not(debug_assertions))]
    pub fn make_node<'a, 'ctx>(&self, nodegen: &NodeGen<'a, 'ctx>) -> SoundNumberInputNode<'ctx> {
        SoundNumberInputNode::new(self.id, nodegen)
    }

    #[cfg(debug_assertions)]
    pub fn make_node<'a, 'ctx>(&self, nodegen: &NodeGen<'a, 'ctx>) -> SoundNumberInputNode<'ctx> {
        SoundNumberInputNode::new(self.id, nodegen, self.scope)
    }
}
