use crate::core::{
    jit::compilednumberinput::CompiledNumberInputFunction,
    number::numberinput::NumberInputId,
    sound::{soundinput::SoundInputId, soundprocessor::SoundProcessorId},
};

use super::{
    stategraph::StateGraph,
    stategraphnode::{NodeTargetValue, SharedProcessorNode},
};

/// Edits to be made to the state graph on the audio thread. These are heavily
/// focused on efficiently inserting pre-allocated state graph data, rather
/// than keeping track of the overall topology. Many edits made at the sound
/// graph level have no analog or may not correspond to any edits here if
/// they don't imply any individual changes to the state graph.
///
/// StateGraphEdits are intended to be produced by the SoundEngine's book-keeping
/// thread as it diffs the existing topology to newly requested topology. When
/// new topology appears, StateGraphEdits are computed which describe the in-place
/// edits to the existing state graph data on the audio thread in order for it to
/// represent the new topology. In-place edits, rather than replacing everything,
/// are required so that existing audio state is preserved and so that audio
/// processing proceeds without audible changes due to an unrelated topological
/// change.
///
/// NOTE: (2024/05/30) the diffing algorithm has not been implimented yet, and
/// updates currently just replace the entire topology at once.
pub(crate) enum StateGraphEdit<'ctx> {
    /// Add a static sound processor node to the state graph. Every static
    /// sound processor can be connected to multiple inputs but always has only
    /// one state. For that reason, every sound processor is allocated exactly
    /// one shared node, at which the most recent chunk of audio is cached.
    ///
    /// All dependent nodes of the static processor, including its number inputs,
    /// sound inputs, any dynamic processor nodes connected to its inputs, and
    /// their further dependencies, are assumed to have been pre-allocated and
    /// to be present in the provided shared processor node already. If there
    /// are any dependencies on static processors, they must be allocated as a
    /// shared state graph node pointing to the same single instance that is
    /// (or will be) present in the state graph as a known static processor.
    ///
    /// After this edit, the shared processor node will begin being evaluated
    /// exactly once per chunk on the audio thread by the SoundEngine.
    AddStaticSoundProcessor(SharedProcessorNode<'ctx>),

    /// Remove a static sound processor node. This also removes all dependent
    /// nodes belonging to the processor, but does not remove any other static
    /// processors it may depend on.
    RemoveStaticSoundProcessor(SoundProcessorId),

    /// Edit an existing sound input within the state graph and add a new
    /// branch to it. Each item in the vector of targets is assumed to contain
    /// all pre-allocated dependencies properly filled as with
    /// AddStaticSoundProcessor. There must be enough targets allocated to
    /// cover all nodes allocated for the given processor.
    // TODO: sound processor id?
    AddSoundInputBranch {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        key_index: usize,
        targets: Vec<NodeTargetValue<'ctx>>,
    },

    /// Remove a branch from all nodes in the state graph for the given
    /// sound input.
    // TODO: sound processor id?
    RemoveSoundInputBranch {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        key_index: usize,
    },

    /// Replace a branch for all nodes in the state graph for the given id.
    /// Like other edits, the vector of targets must enough copies of deeply
    /// pre-allocated data to cover each node in the graph.
    // TODO: sound processor id?
    ReplaceSoundInputBranch {
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        targets: Vec<NodeTargetValue<'ctx>>,
    },

    /// Replace a compiled number input for each number input node in the
    /// graph matching the given id. Compiled functions are Copy, so the
    /// data for each node doesn't need to be separately pre-allocated.
    // TODO: sound processor id?
    UpdateNumberInput(NumberInputId, CompiledNumberInputFunction<'ctx>),

    /// Debugging aid. Calls the given function with the current state graph,
    /// e.g. to test its invariants and whether it matches a desired state.
    /// Does not perform any actual edits and not intended to be used beyond
    /// development and testing.
    DebugInspection(Box<dyn Send + FnOnce(&StateGraph<'ctx>) -> ()>),
}
