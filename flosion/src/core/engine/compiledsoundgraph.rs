use crate::core::sound::soundprocessor::SoundProcessorId;

use super::{
    compiledprocessor::SharedCompiledProcessor,
    compiledsoundgraphedit::CompiledSoundGraphEdit,
    garbage::{Garbage, GarbageChute},
};

/// A directed acyclic graph of nodes representing invidual sound processors,
/// their state, and any cached intermediate outputs. Static processors are
/// always at the top of each sub-graph, and represent a top-level view into
/// other parts of the sub-graph. Dynamic processor nodes which are not
/// shared (cached for re-use) are stored in a Box for unique ownership, while
/// shared/cached nodes are stored in an Arc (for now).
pub struct CompiledSoundGraph<'ctx> {
    static_processors: Vec<SharedCompiledProcessor<'ctx>>,
}

impl<'ctx> CompiledSoundGraph<'ctx> {
    /// Create a new, empty instance
    pub(super) fn new() -> CompiledSoundGraph<'ctx> {
        CompiledSoundGraph {
            static_processors: Vec::new(),
        }
    }

    /// Access the static processor nodes
    pub(super) fn static_processors(&self) -> &[SharedCompiledProcessor<'ctx>] {
        &self.static_processors
    }

    /// Apply an edit, tossing any stale and unwanted
    /// data down the given garbage chute if it could involve heap
    /// deallocation to drop directly.
    pub(super) fn make_edit(
        &mut self,
        edit: CompiledSoundGraphEdit<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        match edit {
            CompiledSoundGraphEdit::AddStaticSoundProcessor(node) => {
                self.add_static_processor(node)
            }
            CompiledSoundGraphEdit::RemoveStaticSoundProcessor(spid) => {
                self.remove_static_processor(spid, garbage_chute)
            }
            CompiledSoundGraphEdit::DebugInspection(f) => f(self),
        }
    }

    /// Add a new static processor node to the graph.
    fn add_static_processor(&mut self, node: SharedCompiledProcessor<'ctx>) {
        debug_assert!(self.static_processors.iter().all(|n| n.id() != node.id()));
        self.static_processors.push(node);
    }

    /// Remove a previously added static processor node from the graph.
    fn remove_static_processor(
        &mut self,
        processor_id: SoundProcessorId,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        debug_assert_eq!(
            self.static_processors
                .iter()
                .filter(|n| n.id() == processor_id)
                .count(),
            1
        );
        let i = self
            .static_processors
            .iter()
            .position(|n| n.id() == processor_id)
            .unwrap();
        let old_node = self.static_processors.remove(i);
        old_node.toss(garbage_chute);
    }
}

impl<'ctx> Garbage<'ctx> for CompiledSoundGraph<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        for proc in self.static_processors {
            proc.toss(chute);
        }
    }
}
