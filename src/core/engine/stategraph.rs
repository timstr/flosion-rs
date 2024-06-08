use crate::core::sound::{soundinput::SoundInputId, soundprocessor::SoundProcessorId};

use super::{
    compiledsoundinput::CompiledSoundInput,
    garbage::{Garbage, GarbageChute},
    stategraphedit::StateGraphEdit,
    stategraphnode::CompiledSoundProcessor,
    stategraphnode::{SharedCompiledProcessor, StateGraphNodeValue},
};

/// A directed acyclic graph of nodes representing invidual sound processors,
/// their state, and any cached intermediate outputs. Static processors are
/// always at the top of each sub-graph, and represent a top-level view into
/// other parts of the sub-graph. Dynamic processor nodes which are not
/// shared (cached for re-use) are stored in a Box for unique ownership, while
/// shared/cached nodes are stored in an Arc (for now).
pub struct StateGraph<'ctx> {
    static_processors: Vec<SharedCompiledProcessor<'ctx>>,
}

impl<'ctx> StateGraph<'ctx> {
    /// Create a new, empty StateGraph instance
    pub(super) fn new() -> StateGraph<'ctx> {
        StateGraph {
            static_processors: Vec::new(),
        }
    }

    /// Access the static processor nodes
    pub(super) fn static_processors(&self) -> &[SharedCompiledProcessor<'ctx>] {
        &self.static_processors
    }

    /// Apply an edit to the StateGraph, tossing any stale and unwanted
    /// data down the given garbage chute if it could involve heap
    /// deallocation to drop directly.
    pub(super) fn make_edit(
        &mut self,
        edit: StateGraphEdit<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        match edit {
            StateGraphEdit::AddStaticSoundProcessor(node) => self.add_static_processor(node),
            StateGraphEdit::RemoveStaticSoundProcessor(spid) => {
                self.remove_static_processor(spid, garbage_chute)
            }
            StateGraphEdit::AddSoundInputBranch {
                input_id,
                owner_id,
                key_index,
                targets,
            } => self.add_sound_input_branch(input_id, key_index, owner_id, targets),
            StateGraphEdit::RemoveSoundInputBranch {
                input_id,
                owner_id,
                key_index,
            } => self.remove_sound_input_branch(input_id, key_index, owner_id, garbage_chute),
            StateGraphEdit::ReplaceSoundInputBranch {
                input_id,
                owner_id,
                targets,
            } => self.replace_sound_input_branch(input_id, owner_id, targets, garbage_chute),
            StateGraphEdit::UpdateExpression(_, _) => todo!(),
            StateGraphEdit::DebugInspection(f) => f(self),
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

    /// Modify all compiled sound inputs corresponding to the given sound input
    /// to add pre-allocated targets at the given branch index. There must
    /// be enough targets allocated for all replicated nodes in the graph.
    /// Internally, this calls `CompiledSoundInput::insert_target`.
    fn add_sound_input_branch(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
        mut targets: Vec<StateGraphNodeValue<'ctx>>,
    ) {
        Self::modify_compiled_sound_inputs(
            &mut self.static_processors,
            owner_id,
            |compiled_input| {
                compiled_input.insert(input_id, index, targets.pop().unwrap());
            },
        );
    }

    /// Modify all compiled sound inputs corresponding to the given sound input
    /// to remove targets at the given branch index. The removed targets are
    /// all tossed into the given GarbageChute. Internally, this calls
    /// `CompiledSoundInput::erase_target`
    fn remove_sound_input_branch(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        Self::modify_compiled_sound_inputs(
            &mut self.static_processors,
            owner_id,
            |compiled_input| {
                let old_target = compiled_input.erase(input_id, index);
                old_target.toss(garbage_chute);
            },
        );
    }

    /// Modify all compiled sound inputs corresponding to the given sound input
    /// to swap their targets in-place with the given, pre-allocated targets.
    /// The removed targets are all tossed into the given GarbageChute. There
    /// must be enough targets allocated for all replicated nodes in the graph.
    fn replace_sound_input_branch(
        &mut self,
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        mut targets: Vec<StateGraphNodeValue<'ctx>>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        Self::modify_compiled_sound_inputs(
            &mut self.static_processors,
            owner_id,
            |compiled_input| {
                for target in compiled_input.targets_mut() {
                    if target.id() != input_id {
                        continue;
                    }
                    let old_target = target.swap_target(targets.pop().unwrap());
                    old_target.toss(garbage_chute);
                }
            },
        );
        // TODO: toss the vec also
    }

    /// Internal helper method for looking up all copies of and making
    /// changes to the nodes of a sound input in the StateGraph.
    fn modify_compiled_sound_inputs<F: FnMut(&mut dyn CompiledSoundInput<'ctx>)>(
        _static_nodes: &mut [SharedCompiledProcessor<'ctx>],
        _owner_id: SoundProcessorId,
        _f: F,
    ) {
        todo!()
    }

    /// Internal helper method for looking up all copies of and making
    /// changes to the nodes of a sound processor in the StateGraph.
    fn modify_compiled_processors<F: FnMut(&mut dyn CompiledSoundProcessor<'ctx>)>(
        _static_nodes: &mut [SharedCompiledProcessor<'ctx>],
        _processor_id: SoundProcessorId,
        _f: F,
    ) {
        todo!()
    }
}

impl<'ctx> Garbage<'ctx> for StateGraph<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        for proc in self.static_processors {
            proc.toss(chute);
        }
    }
}
