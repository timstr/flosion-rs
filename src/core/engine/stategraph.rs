use crate::core::sound::{
    soundgraphdata::SoundNumberInputData, soundinput::SoundInputId, soundinputnode::SoundInputNode,
    soundnumberinput::SoundNumberInputId, soundprocessor::SoundProcessorId,
};

use super::{
    garbage::{Garbage, GarbageChute},
    stategraphedit::StateGraphEdit,
    stategraphnode::StateGraphNode,
    stategraphnode::{NodeTargetValue, SharedProcessorNode},
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

    pub(super) fn make_edit(
        &mut self,
        edit: StateGraphEdit<'ctx>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        match edit {
            StateGraphEdit::AddStaticSoundProcessor(data) => self.add_static_sound_processor(data),
            StateGraphEdit::RemoveStaticSoundProcessor(spid) => {
                self.remove_static_sound_processor(spid, garbage_chute)
            }
            StateGraphEdit::AddSoundInput {
                input_id,
                owner,
                targets,
            } => self.add_sound_input(input_id, owner, targets),
            StateGraphEdit::RemoveSoundInput { input_id, owner_id } => {
                self.remove_sound_input(input_id, owner_id, garbage_chute)
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
            } => self.remove_sound_input_key(input_id, key_index, owner_id, garbage_chute),
            StateGraphEdit::ReplaceSoundInputTargets {
                input_id,
                owner_id,
                targets,
            } => self.replace_sound_input_targets(input_id, owner_id, targets, garbage_chute),
            StateGraphEdit::UpdateNumberInput(_, _) => todo!(),
            StateGraphEdit::DebugInspection(f) => f(self),
        }
    }

    fn add_static_sound_processor(&mut self, node: Box<dyn 'ctx + StateGraphNode<'ctx>>) {
        let shared_node = SharedProcessorNode::<'ctx>::new(node);
        self.static_nodes.push(shared_node);
    }

    fn remove_static_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        debug_assert_eq!(
            self.static_nodes
                .iter()
                .filter(|n| n.id() == processor_id)
                .count(),
            1
        );
        let i = self
            .static_nodes
            .iter()
            .position(|n| n.id() == processor_id)
            .unwrap();
        let old_node = self.static_nodes.remove(i);
        old_node.toss(garbage_chute);
    }

    fn add_sound_input(
        &mut self,
        input_id: SoundInputId,
        owner: SoundProcessorId,
        mut targets: Vec<NodeTargetValue<'ctx>>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner, |node| {
            let key_index = 0;
            node.insert_target(input_id, key_index, targets.pop().unwrap());
        });
    }

    fn remove_sound_input(
        &mut self,
        input_id: SoundInputId,
        owner: SoundProcessorId,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner, |node| {
            let key_index = 0;
            let old_target = node.erase_target(input_id, key_index);
            old_target.toss(garbage_chute);
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
            node.insert_target(input_id, index, targets.pop().unwrap());
        });
    }

    fn remove_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            let old_target = node.erase_target(input_id, index);
            old_target.toss(garbage_chute);
        });
    }

    fn add_number_input(&mut self, _data: SoundNumberInputData) {
        todo!();
    }

    fn remove_number_input(&mut self, _input_id: SoundNumberInputId) {
        todo!();
    }

    fn replace_sound_input_targets(
        &mut self,
        input_id: SoundInputId,
        owner_id: SoundProcessorId,
        mut targets: Vec<NodeTargetValue<'ctx>>,
        garbage_chute: &GarbageChute<'ctx>,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            for target in node.targets_mut() {
                if target.id() != input_id {
                    continue;
                }
                let old_target = target.swap_target(targets.pop().unwrap());
                old_target.toss(garbage_chute);
            }
        });
        // TODO: toss the vec also
    }

    fn modify_sound_input_node<F: FnMut(&mut dyn SoundInputNode<'ctx>)>(
        _static_nodes: &mut [SharedProcessorNode<'ctx>],
        _owner_id: SoundProcessorId,
        _f: F,
    ) {
        todo!()
    }

    fn modify_processor_node<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(
        _static_nodes: &mut [SharedProcessorNode<'ctx>],
        _processor_id: SoundProcessorId,
        _f: F,
    ) {
        todo!()
    }
}

impl<'ctx> Garbage<'ctx> for StateGraph<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        for node in self.static_nodes {
            node.toss(chute);
        }
    }
}
