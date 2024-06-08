use crate::core::{
    engine::{
        soundgraphcompiler::SoundGraphCompiler,
        stategraphnode::{CompiledSoundInputBranch, StateGraphNodeValue},
    },
    sound::soundinput::SoundInputId,
};

/// CompiledSoundInput is a trait for accessing and modifying the state
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
pub trait CompiledSoundInput<'ctx>: Sync + Send {
    /// Access the targets of the sound input node
    fn targets(&self) -> &[CompiledSoundInputBranch<'ctx>];

    /// Mutably access the targets of the sound input node
    fn targets_mut(&mut self) -> &mut [CompiledSoundInputBranch<'ctx>];

    /// Add the first branch for a newly-created sound input or add a branch
    /// to an existing set of branches under an existing input. The exact
    /// interpretation of what this means is largely up to the implementing
    /// sound input type.
    fn insert(
        &mut self,
        _input_id: SoundInputId,
        _key_index: usize,
        _value: StateGraphNodeValue<'ctx>,
    ) {
        panic!("This input node type does not support inserting inputs or branches");
    }

    /// Remove a target from an existing set of branches, or remove an
    /// existing sound input entirely. The exact interpretation is similarly
    /// up to the implementing sound input type.
    fn erase(&mut self, _input_id: SoundInputId, _key_index: usize) -> StateGraphNodeValue<'ctx> {
        panic!("This input node type does not support erasing inputs or branches");
    }
}

/// The unit type `()` can be used as a CompiledSoundInput with no targets
impl<'ctx> CompiledSoundInput<'ctx> for () {
    fn targets(&self) -> &[CompiledSoundInputBranch<'ctx>] {
        &[]
    }

    fn targets_mut(&mut self) -> &mut [CompiledSoundInputBranch<'ctx>] {
        &mut []
    }
}

/// SoundProcessorInput is a trait for the sound input used by a given
/// sound processor. While the trait itself is concerned mainly with
/// allocating nodes for the StateGraph, actual types implementing this
/// trait will typically provide diverse and fully-featured APIs for
/// using different types of sound inputs. See implementations for more.
pub trait SoundProcessorInput: Sync + Send {
    type NodeType<'ctx>: CompiledSoundInput<'ctx>;

    fn make_node<'a, 'ctx>(
        &self,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::NodeType<'ctx>;

    fn list_ids(&self) -> Vec<SoundInputId>;
}
