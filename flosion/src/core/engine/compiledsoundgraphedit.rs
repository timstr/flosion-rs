use crate::core::sound::soundprocessor::SoundProcessorId;

use super::{compiledprocessor::SharedCompiledProcessor, compiledsoundgraph::CompiledSoundGraph};

/// Edits to be made to the compiled sound graph on the audio thread. These are heavily
/// focused on efficiently inserting pre-allocated data, rather
/// than keeping track of the overall graph. Many edits made at the sound
/// graph level have no analog or may not correspond to any edits here if
/// they don't imply any individual changes to the compiled sound graph.
///
/// These edits are intended to be produced by the SoundEngine's book-keeping
/// thread as it diffs the existing graph to newly requested graph. When
/// a new graph appears, edits are computed which describe the in-place
/// edits to the existing compiled data on the audio thread in order for it to
/// represent the new graph. In-place edits, rather than replacing everything,
/// are required so that existing audio state is preserved and so that audio
/// processing proceeds without audible changes due to an unrelated graph
/// change.
///
/// NOTE: (2024/05/30) the diffing algorithm has not been implemented yet, and
/// updates currently just replace the entire graph at once.
pub(crate) enum CompiledSoundGraphEdit<'ctx> {
    /// Add a static sound processor node. Every static
    /// sound processor can be connected to multiple inputs but always has only
    /// one state. For that reason, every sound processor is allocated exactly
    /// one shared node, at which the most recent chunk of audio is cached.
    ///
    /// All dependent nodes of the static processor, including its expressions,
    /// sound inputs, any dynamic processor nodes connected to its inputs, and
    /// their further dependencies, are assumed to have been pre-allocated and
    /// to be present in the provided shared processor node already. If there
    /// are any dependencies on static processors, they must be allocated as a
    /// shared node pointing to the same single instance that is
    /// (or will be) present in the compiled sound graph as a known static processor.
    ///
    /// After this edit, the shared processor node will begin being evaluated
    /// exactly once per chunk on the audio thread by the SoundEngine.
    AddStaticSoundProcessor(SharedCompiledProcessor<'ctx>),

    /// Remove a static sound processor node. This also removes all dependent
    /// nodes belonging to the processor, but does not remove any other static
    /// processors it may depend on.
    RemoveStaticSoundProcessor(SoundProcessorId),

    /// Debugging aid. Calls the given function with the current compiled sound graph,
    /// e.g. to test its invariants and whether it matches a desired state.
    /// Does not perform any actual edits and not intended to be used beyond
    /// development and testing.
    DebugInspection(Box<dyn Send + FnOnce(&CompiledSoundGraph<'ctx>) -> ()>),
}
