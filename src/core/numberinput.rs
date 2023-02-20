use std::slice;

use super::{
    context::Context, numberinputnode::NumberInputNode, numbersource::NumberSourceId,
    soundprocessor::SoundProcessorId, state::StateOwner, uniqueid::UniqueId,
};

// TODO: consider making usize field private, prefer .value() over .0
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
pub(crate) enum NumberInputOwner {
    SoundProcessor(SoundProcessorId),
    NumberSource(NumberSourceId),
}

impl NumberInputOwner {
    pub(super) fn is_stateful(&self) -> bool {
        match self {
            NumberInputOwner::SoundProcessor(_) => true,
            NumberInputOwner::NumberSource(_) => false,
        }
    }

    pub(super) fn as_state_owner(&self) -> Option<StateOwner> {
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
    pub(crate) fn new(id: NumberInputId, owner: NumberInputOwner) -> NumberInputHandle {
        NumberInputHandle { id, owner }
    }

    pub fn id(&self) -> NumberInputId {
        self.id
    }

    pub(super) fn owner(&self) -> NumberInputOwner {
        self.owner
    }

    pub fn make_node<'ctx>(
        &self,
        _context: &'ctx inkwell::context::Context,
    ) -> NumberInputNode<'ctx> {
        NumberInputNode::new(self.id)
    }

    pub fn interpret(&self, dst: &mut [f32], context: &Context) {
        context.interpret_number_input(self.id, dst);
    }

    pub fn interpret_scalar(&self, context: &Context) -> f32 {
        let mut x: f32 = 0.0;
        context.interpret_number_input(self.id, slice::from_mut(&mut x));
        x
    }
}
