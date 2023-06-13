use crate::core::engine::stategraphnode::{NodeTarget, OpaqueNodeTargetValue};

use super::soundinput::SoundInputId;

// TODO: move to core::engine

// Trait used for automating allocation and reallocation of node inputs
// Not concerned with actual audio processing or providing access to
// said inputs - concrete types will provide those.
pub trait SoundInputNode<'ctx> {
    // TODO: delete add_input, remove_input, add_key, remove_key, replace
    // them with functions that the state graph and audio processing directly
    // care about, e.g. replacing NodeTargetValues. That's it! Replacing
    // NodeTargetValues should be general enough to suffice for all adding
    // and removing of inputs and their keys while also highly efficient
    // if targets are pre-allocated in the right quantity.
    //
    // Operations currently needed:
    //   single input:
    //    - replace target
    //    - NO adding/removing input ids necessary
    //    - NO adding/removing keys necessary, only ever 1 effective key
    //    - NO additional state
    //   single input list:
    //    - add input id with target    !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!1
    //    - remove input id and its target  !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!1
    //    - replace target by input id
    //    - NO adding/removing keys necessary, only ever 1 effective key per input id
    //    - NO additional state
    //   keyed input (multi input / keyed input queue):
    //    - replace targets of all keys
    //    - insert key with target
    //    - erase key with target
    //    - NO adding/removing input ids necessary
    //    - possible additional state for each key.
    // Ignoring keyed input states, the following three operations should suffice for
    // sound input node use cases as listed above:
    //  - replace a set of targets by input id and key index (e.g. all targets when reconnecting any input)
    //  - insert a target by input id and key index (keyed input only)
    //  - erase a target by input id and key index (keyed input only)
    // These operations will focus on directly providing all pre-allocated node targets whenever needed.
    //
    // Access to the targets will still be needed for recursing through the state graph, for which a simple
    // reference to a slice would be very easy to deal with and conceptually possible for all current input
    // types. Additionally, a mutable slice of input types could be used to replace all targets without
    // needing to implement that behaviour in all input node types.
    //
    // One unaddressed problem is that arbitrary length lists of sound inputs and sound input keys may still
    // at some point require re-allocation while preserving previous node targets. For now, a simple Vec
    // that on occasion reallocates on the audio thread should be fine. In the future, a linked list may
    // actually be viable here, but will need performance testing to justify its complexity.
    //
    // How to pre-allocate custom keyed input data? Does it need pre-allocating at all?
    //  - all keyed input states could be stored in a single flat array, removing the
    //    need for any additional indirection
    //  - that array's reallocation issues are essentially the same as those of the array of node targets
    //    (moot for now, solvable with a fancy realtime linked list data structure later if critically necessary)
    //  - The sound input node instance can allocate default versions of its state type whenever receiving
    //    an insert command.
    //  - States should probably be Copy
    //  - Input states can be treated very similarly to sound processor states, in that they get default
    //    constructed and can be reset, and the sound processor's audio processing routine can rely on
    //    custom book-keeping information to track when something was added or reset if it needs to

    // fn add_input(&mut self, _input_id: SoundInputId);

    // fn remove_input(&mut self, _input_id: SoundInputId);

    // fn add_key(&mut self, _input_id: SoundInputId, _index: usize) {
    //     panic!("This input node type does not support keys");
    // }

    // fn remove_key(&mut self, _input_id: SoundInputId, _index: usize) {
    //     panic!("This input node type does not support keys");
    // }

    fn targets(&self) -> &[NodeTarget<'ctx>];
    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>];

    fn insert_target(
        &mut self,
        input_id: SoundInputId,
        key_index: usize,
        target: OpaqueNodeTargetValue<'ctx>,
    ) {
        panic!("This input node type does not support inserting targets");
    }

    fn erase_target(
        &mut self,
        input_id: SoundInputId,
        key_index: usize,
    ) -> OpaqueNodeTargetValue<'ctx> {
        panic!("This input node type does not support erasing targets");
    }
}

impl<'ctx> SoundInputNode<'ctx> for () {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        &[]
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        &mut []
    }
}

pub trait SoundProcessorInput {
    type NodeType<'ctx>: SoundInputNode<'ctx>;

    fn make_node<'ctx>(&self) -> Self::NodeType<'ctx>;

    fn list_ids(&self) -> Vec<SoundInputId>;
}
