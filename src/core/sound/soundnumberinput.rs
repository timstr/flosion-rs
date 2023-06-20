use crate::core::{engine::nodegen::NodeGen, uniqueid::UniqueId};

use super::{soundnumberinputnode::SoundNumberInputNode, soundprocessor::SoundProcessorId};

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
}

impl SoundNumberInputHandle {
    pub fn new(id: SoundNumberInputId, owner: SoundProcessorId) -> SoundNumberInputHandle {
        SoundNumberInputHandle { id, owner }
    }

    pub fn id(&self) -> SoundNumberInputId {
        self.id
    }

    pub fn make_node<'a, 'ctx>(&self, nodegen: &NodeGen<'a, 'ctx>) -> SoundNumberInputNode<'ctx> {
        SoundNumberInputNode::new(self.id, nodegen)
    }
}
