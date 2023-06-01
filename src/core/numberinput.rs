use super::{numbersource::NumberSourceId, uniqueid::UniqueId};

// TODO: rework this, make number inputs for sound processors and number graphs separate

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberInputId(usize);

impl Default for NumberInputId {
    fn default() -> NumberInputId {
        NumberInputId(1)
    }
}

impl UniqueId for NumberInputId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> NumberInputId {
        NumberInputId(self.0 + 1)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum NumberInputOwner {
    NumberSource(NumberSourceId),
    ParentGraph,
}

pub struct NumberInputHandle {
    id: NumberInputId,
    owner: NumberInputOwner,
}

impl NumberInputHandle {
    pub(crate) fn new(id: NumberInputId, owner: NumberInputOwner) -> NumberInputHandle {
        NumberInputHandle { id, owner }
    }

    pub fn id(&self) -> NumberInputId {
        self.id
    }

    pub(super) fn owner(&self) -> NumberInputOwner {
        self.owner
    }
}
