use crate::core::{
    engine::{
        nodegen::NodeGen,
        stategraphnode::{NodeTarget, NodeTargetValue},
    },
    sound::soundinput::SoundInputId,
};

/// SoundInputNode is a trait for accessing and modifying the state
/// graph nodes associated with a sound input or collection of sound
/// inputs inside of a StateGraph. Generally, this will only be internally
/// implemented by the node types of specific sound input types, such as
/// SingleInput, QueuedInput, etc.
///
/// The required methods are `targets` and `targets_mut` which give access
/// to the stored state graph nodes. The optional methods `insert_target`
/// and `erase_target` are only needed for sound inputs that support adding
/// and removing additional inputs or branches per input after the parent
/// sound processor has been constructed.
pub trait SoundInputNode<'ctx>: Sync + Send {
    /// Access the targets of the sound input node
    fn targets(&self) -> &[NodeTarget<'ctx>];

    /// Mutably access the targets of the sound input node
    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>];

    /// Add the first branch for a newly-created sound input or add a branch
    /// to an existing set of branches under an existing input. The exact
    /// interpretation of what this means is largely up to the implementing
    /// sound input type.
    fn insert_target(
        &mut self,
        _input_id: SoundInputId,
        _key_index: usize,
        _target: NodeTargetValue<'ctx>,
    ) {
        panic!("This input node type does not support inserting targets");
    }

    /// Remove a target from an existing set of branches, or remove an
    /// existing sound input entirely. The exact interpretation is similarly
    /// up to the implementing sound input type.
    fn erase_target(
        &mut self,
        _input_id: SoundInputId,
        _key_index: usize,
    ) -> NodeTargetValue<'ctx> {
        panic!("This input node type does not support erasing targets");
    }
}

/// The unit type `()` can be used as a SoundInputNode with no targets
impl<'ctx> SoundInputNode<'ctx> for () {
    fn targets(&self) -> &[NodeTarget<'ctx>] {
        &[]
    }

    fn targets_mut(&mut self) -> &mut [NodeTarget<'ctx>] {
        &mut []
    }
}

/// SoundProcessorInput is a trait for the sound input used by a given
/// sound processor. While the trait itself is concerned mainly with
/// allocating nodes for the StateGraph, actual types implementing this
/// trait will typically provide diverse and fully-featured APIs for
/// using different types of sound inputs. See implementations for more.
pub trait SoundProcessorInput: Sync + Send {
    type NodeType<'ctx>: SoundInputNode<'ctx>;

    fn make_node<'a, 'ctx>(&self, nodegen: &mut NodeGen<'a, 'ctx>) -> Self::NodeType<'ctx>;

    fn list_ids(&self) -> Vec<SoundInputId>;
}
