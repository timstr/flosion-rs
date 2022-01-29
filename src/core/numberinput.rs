use core::slice;

use super::{
    context::NumberContext, numbersource::NumberSourceId, soundprocessor::SoundProcessorId,
    soundstate::StateOwner, uniqueid::UniqueId,
};

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
pub enum NumberInputOwner {
    SoundProcessor(SoundProcessorId),
    NumberSource(NumberSourceId),
}

impl NumberInputOwner {
    pub fn is_stateful(&self) -> bool {
        match self {
            NumberInputOwner::SoundProcessor(_) => true,
            NumberInputOwner::NumberSource(_) => false,
        }
    }

    pub fn as_state_owner(&self) -> Option<StateOwner> {
        match self {
            NumberInputOwner::SoundProcessor(spid) => Some(StateOwner::SoundProcessor(*spid)),
            NumberInputOwner::NumberSource(_) => None,
        }
    }
}

pub struct NumberInputHandle {
    id: NumberInputId,
    owner: NumberInputOwner,
}

impl NumberInputHandle {
    pub fn new(id: NumberInputId, owner: NumberInputOwner) -> NumberInputHandle {
        NumberInputHandle { id, owner }
    }

    pub fn id(&self) -> NumberInputId {
        self.id
    }

    pub fn owner(&self) -> NumberInputOwner {
        self.owner
    }

    pub fn eval(&self, dst: &mut [f32], context: NumberContext) {
        context.evaluate_input(self.id, dst);
    }

    pub fn eval_scalar(&self, context: NumberContext) -> f32 {
        let mut x: f32 = 0.0;
        context.evaluate_input(self.id, slice::from_mut(&mut x));
        x
    }

    pub(super) fn clone(&self) -> NumberInputHandle {
        NumberInputHandle {
            id: self.id,
            owner: self.owner,
        }
    }
}
