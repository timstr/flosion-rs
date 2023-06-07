use crate::core::uniqueid::UniqueId;

use super::numbersource::NumberSourceId;

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

pub struct NumberInputHandle {
    id: NumberInputId,
    owner: NumberSourceId,
}

impl NumberInputHandle {
    pub(crate) fn new(id: NumberInputId, owner: NumberSourceId) -> NumberInputHandle {
        NumberInputHandle { id, owner }
    }

    pub fn id(&self) -> NumberInputId {
        self.id
    }

    pub(super) fn owner(&self) -> NumberSourceId {
        self.owner
    }
}
