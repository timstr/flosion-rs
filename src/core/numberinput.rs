use std::slice;

use super::{
    context::Context, numbersource::NumberSourceId, soundprocessor::SoundProcessorId,
    statetree::StateOwner, uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberInputId(pub usize);

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

    pub fn make_node(&self) -> NumberInputNode {
        NumberInputNode::new(self.id)
    }

    pub fn eval(&self, dst: &mut [f32], context: &Context) {
        context.evaluate_number_input(self.id, dst);
    }

    pub fn eval_scalar(&self, context: &Context) -> f32 {
        let mut x: f32 = 0.0;
        context.evaluate_number_input(self.id, slice::from_mut(&mut x));
        x
    }
}

pub struct NumberInputNode {
    id: NumberInputId,
}

impl NumberInputNode {
    pub(super) fn new(id: NumberInputId) -> Self {
        Self { id }
    }

    pub fn eval(&self, dst: &mut [f32], context: &Context) {
        context.evaluate_number_input(self.id, dst);
    }

    pub fn eval_scalar(&self, context: &Context) -> f32 {
        let mut dst: f32 = 0.0;
        let s = slice::from_mut(&mut dst);
        context.evaluate_number_input(self.id, s);
        dst
    }
}
