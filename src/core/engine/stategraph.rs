use crate::core::sound::{
    soundgraphdata::SoundNumberInputData, soundinput::SoundInputId, soundinputnode::SoundInputNode,
    soundnumberinput::SoundNumberInputId, soundprocessor::SoundProcessorId,
};

use super::{
    stategraphedit::StateGraphEdit,
    stategraphnode::{NodeTargetValue, SharedProcessorNode},
    stategraphnode::{OpaqueNodeTargetValue, StateGraphNode},
};

// A directed acyclic graph of nodes representing invidual sound processors,
// their state, and any cached intermediate outputs. Static processors are
// always at the top of each sub-graph, and represent a top-level view into
// other parts of the sub-graph. Dynamic processor nodes which are not
// shared (cached for re-use) are stored in a Box for unique ownership, while
// shared/cached nodes are stored in an Arc (for now).

pub struct StateGraph<'ctx> {
    static_nodes: Vec<SharedProcessorNode<'ctx>>,
}

impl<'ctx> StateGraph<'ctx> {
    pub(super) fn new() -> StateGraph<'ctx> {
        StateGraph {
            static_nodes: Vec::new(),
        }
    }

    pub(super) fn static_nodes(&self) -> &[SharedProcessorNode<'ctx>] {
        &self.static_nodes
    }

    pub(super) fn make_edit(&mut self, edit: StateGraphEdit<'ctx>) {
        // TODO: consider adding a special edit type which marks that a batch of updates
        // is over, in order to avoid keeping all invariants in check unecessarily (e.g.
        // recompiling number inputs over and over). This would go nicely with marking
        // number inputs as "dirty" until updated
        match edit {
            StateGraphEdit::AddStaticSoundProcessor(data) => self.add_static_sound_processor(data),
            StateGraphEdit::RemoveStaticSoundProcessor(spid) => {
                self.remove_static_sound_processor(spid)
            }
            StateGraphEdit::AddSoundInput {
                input_id,
                owner,
                targets,
            } => self.add_sound_input(input_id, owner, targets),
            StateGraphEdit::RemoveSoundInput { input_id, owner_id } => {
                self.remove_sound_input(input_id, owner_id)
            }
            StateGraphEdit::AddSoundInputKey {
                input_id,
                owner_id,
                key_index,
                targets,
            } => self.add_sound_input_key(input_id, key_index, owner_id, targets),
            StateGraphEdit::RemoveSoundInputKey {
                input_id,
                owner_id,
                key_index,
            } => self.remove_sound_input_key(input_id, key_index, owner_id),
            StateGraphEdit::ReplaceSoundInputTargets {
                input_id,
                owner_id,
                targets,
            } => self.replace_sound_input_targets(input_id, owner_id, targets),
            StateGraphEdit::UpdateNumberInput(_, _) => todo!(),
            StateGraphEdit::DebugInspection(f) => f(self),
        }
    }

    fn add_static_sound_processor(&mut self, node: Box<dyn 'ctx + StateGraphNode<'ctx>>) {
        let shared_node = SharedProcessorNode::<'ctx>::new(node);
        self.static_nodes.push(shared_node);
    }

    fn remove_static_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // NOTE: the processor is assumed to already be completely
        // disconnected. Dynamic processor nodes will thus have no
        // nodes allocated, and only static processors will have
        // allocated nodes.
        // TODO: throw removed node into the garbage chute
        self.static_nodes.retain(|n| n.id() != processor_id);
    }

    fn add_sound_input(
        &mut self,
        input_id: SoundInputId,
        owner: SoundProcessorId,
        mut targets: Vec<NodeTargetValue<'ctx>>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner, |node| {
            let key_index = 0;
            node.insert_target(
                input_id,
                key_index,
                OpaqueNodeTargetValue(targets.pop().unwrap()),
            );
        });
    }

    fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner, |node| {
            let key_index = 0;
            let old_target = node.erase_target(input_id, key_index);
            // TODO: put old_target in the garbage chute
        });
    }

    fn add_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
        mut targets: Vec<NodeTargetValue<'ctx>>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            node.insert_target(
                input_id,
                index,
                OpaqueNodeTargetValue(targets.pop().unwrap()),
            );
        });
    }

    fn remove_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            let old_target = node.erase_target(input_id, index);
            // TODO: put old_target in the garbage chute
        });
    }

    fn add_number_input(&mut self, data: SoundNumberInputData) {
        todo!();
    }

    fn remove_number_input(&mut self, input_id: SoundNumberInputId) {
        todo!();
    }

    fn replace_sound_input_targets(
        &mut self,
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        mut targets: Vec<NodeTargetValue<'ctx>>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            for target in node.targets_mut() {
                target.set_target(targets.pop().unwrap());
            }
        });
    }

    fn modify_sound_input_node<F: FnMut(&mut dyn SoundInputNode<'ctx>)>(
        static_nodes: &mut [SharedProcessorNode<'ctx>],
        owner_id: SoundProcessorId,
        mut f: F,
    ) {
        todo!()
    }

    fn modify_processor_node<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(
        static_nodes: &mut [SharedProcessorNode<'ctx>],
        processor_id: SoundProcessorId,
        mut f: F,
    ) {
        todo!()
    }
}
