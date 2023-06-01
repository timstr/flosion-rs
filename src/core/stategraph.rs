use std::{collections::HashSet, marker::PhantomData};

use inkwell::context::Context;

use crate::core::soundinput::InputTiming;

use super::{
    numberinputnode::SoundNumberInputNode,
    soundgraphdata::{
        SoundInputData, SoundNumberInputData, SoundNumberSourceData, SoundProcessorData,
    },
    soundgraphedit::SoundGraphEdit,
    soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId,
    soundinputnode::{SoundInputNode, SoundInputNodeVisitorMut},
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId,
    soundprocessor::SoundProcessorId,
    stategraphnode::StateGraphNode,
    stategraphnode::{NodeTarget, NodeTargetValue},
    stategraphnode::{SharedProcessorNode, UniqueProcessorNode},
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
        edit: SoundGraphEdit,
        topology: &SoundGraphTopology,
        context: &'ctx Context,
    ) {
        // TODO: consider adding a special edit type which marks that a batch of updates
        // is over, in order to avoid keeping all invariants in check unecessarily (e.g.
        // recompiling number inputs over and over). This would go nicely with marking
        // number inputs as "dirty" until updated
        match edit {
            SoundGraphEdit::AddSoundProcessor(data) => self.add_sound_processor(data, context),
            SoundGraphEdit::RemoveSoundProcessor(spid) => self.remove_sound_processor(spid),
            SoundGraphEdit::AddSoundInput(data) => self.add_sound_input(data),
            SoundGraphEdit::RemoveSoundInput(siid, owner) => self.remove_sound_input(siid, owner),
            SoundGraphEdit::AddSoundInputKey(siid, i) => {
                self.add_sound_input_key(siid, i, topology.sound_input(siid).unwrap().owner())
            }
            SoundGraphEdit::RemoveSoundInputKey(siid, i) => {
                self.remove_sound_input_key(siid, i, topology.sound_input(siid).unwrap().owner())
            }
            SoundGraphEdit::ConnectSoundInput(siid, spid) => {
                self.connect_sound_input(siid, spid, topology, context)
            }
            SoundGraphEdit::DisconnectSoundInput(siid) => {
                self.disconnect_sound_input(siid, topology)
            }
            SoundGraphEdit::AddNumberSource(data) => self.add_number_source(data),
            SoundGraphEdit::RemoveNumberSource(nsid, _owner) => self.remove_number_source(nsid),
            SoundGraphEdit::AddNumberInput(data) => self.add_number_input(data, topology, context),
            SoundGraphEdit::RemoveNumberInput(niid, _owner) => {
                self.remove_number_input(niid, topology, context)
            }
            SoundGraphEdit::ConnectNumberInput(niid, nsid) => {
                self.connect_number_input(niid, nsid, topology, context)
            }
            SoundGraphEdit::DisconnectNumberInput(niid, nsid) => {
                self.disconnect_number_input(niid, nsid, topology, context)
            }
        }
    }

    fn add_sound_processor<'a>(&'a mut self, data: SoundProcessorData, context: &'ctx Context) {
        // if the processor is static, add it as an entry point
        // otherwise, since it isn't connected to anything
        // and thus won't be called, nothing needs doing
        if !data.instance().is_static() {
            return;
        }
        let node = data.instance_arc().make_node(context);
        let shared_node = SharedProcessorNode::<'ctx>::new(node);
        self.static_nodes.push(shared_node);
    }

    fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // NOTE: the processor is assumed to already be completely
        // disconnected. Dynamic processor nodes will thus have no
        // nodes allocated, and only static processors will have
        // allocated nodes.
        self.static_nodes.retain(|n| n.id() != processor_id);
    }

    fn add_sound_input(&mut self, data: SoundInputData) {
        Self::modify_sound_input_node(&mut self.static_nodes, data.owner(), |node| {
            node.add_input(data.id());
        });
    }

    fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner, |node| {
            node.remove_input(input_id);
        });
    }

    fn add_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            node.add_key(input_id, index);
        });
    }

    fn remove_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
    ) {
        Self::modify_sound_input_node(&mut self.static_nodes, owner_id, |node| {
            node.remove_key(input_id, index);
        });
    }

    fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
        topology: &SoundGraphTopology,
        context: &'ctx Context,
    ) {
        let input_data = topology.sound_input(input_id).unwrap();

        // TODO: allocate these not on the audio thread
        // TODO: make this context-aware so that it detects reused nodes in a synchronous
        // group and caches them.
        // For now, no caching...

        let num_input_nodes = self.count_input_nodes(input_data.id());
        let mut targets = Vec::new();
        for _ in 0..num_input_nodes {
            targets.push(Self::allocate_subgraph(
                &self.static_nodes,
                processor_id,
                topology,
                context,
            ));
        }

        Self::modify_sound_input_node(&mut self.static_nodes, input_data.owner(), |node| {
            node.visit_inputs_mut(
                &mut |siid: SoundInputId,
                      _kidx: usize,
                      tgt: &mut NodeTarget<'ctx>,
                      timing: &mut InputTiming| {
                    if siid == input_id {
                        debug_assert!(tgt.is_empty());
                        tgt.set_target(targets.pop().unwrap());
                        timing.require_reset();
                    }
                },
            );
        });

        debug_assert!(targets.is_empty());
    }

    fn disconnect_sound_input(&mut self, input_id: SoundInputId, topology: &SoundGraphTopology) {
        let input_data = topology.sound_input(input_id).unwrap();
        Self::modify_sound_input_node(&mut self.static_nodes, input_data.owner(), |node| {
            node.visit_inputs_mut(
                &mut |_siid: SoundInputId,
                      _kidx: usize,
                      tgt: &mut NodeTarget,
                      timing: &mut InputTiming| {
                    debug_assert!(!tgt.is_empty());
                    tgt.set_target(NodeTargetValue::Empty);
                    timing.mark_as_done();
                },
            )
        });
    }

    fn add_number_source(&mut self, _data: SoundNumberSourceData) {
        // The number source is not connected to anything,
        // and so can't be evaluated and can't trigger a re-compile
    }

    fn remove_number_source(&mut self, _source_id: SoundNumberSourceId) {
        // similar to add_number_source
    }

    fn add_number_input(
        &mut self,
        data: SoundNumberInputData,
        topology: &SoundGraphTopology,
        context: &'ctx Context,
    ) {
        // add a number input node to the owner processor's nodes
        // but leave them empty - the new number input can't
        // be connected to anything yet.
        Self::modify_processor_node(
            &mut self.static_nodes,
            data.owner(),
            |node: &mut dyn StateGraphNode<'ctx>| {
                node.number_input_node_mut().add_input(data.id());
                node.visit_number_inputs_mut(&mut |input: &mut SoundNumberInputNode<'ctx>| {
                    if input.id() == data.id() {
                        // TODO: do this not on the audio thread
                        input.recompile(topology, context);
                    }
                });
            },
        );
    }

    fn remove_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        topology: &SoundGraphTopology,
        context: &'ctx Context,
    ) {
        let data = topology.number_input(input_id).unwrap();

        Self::modify_processor_node(
            &mut self.static_nodes,
            data.owner(),
            |node: &'_ mut dyn StateGraphNode<'ctx>| {
                node.number_input_node_mut().remove_input(input_id);
                node.visit_number_inputs_mut(&mut |input: &mut SoundNumberInputNode<'ctx>| {
                    if input.id() == input_id {
                        // TODO: do this not on the audio thread
                        input.recompile(topology, context);
                    }
                });
            },
        );
    }

    fn connect_number_input(
        &mut self,
        input_id: SoundNumberInputId,
        source_id: SoundNumberSourceId,
        topology: &SoundGraphTopology,
        context: &'ctx inkwell::context::Context,
    ) {
        // HACK: recompiling all number inputs for now
        let dependents = topology.number_inputs().keys().cloned().collect();
        self.recompile_number_input_nodes(&dependents, topology, context);
    }

    fn disconnect_number_input(
        &mut self,
        _input_id: SoundNumberInputId,
        _source_id: SoundNumberSourceId,
        topology: &SoundGraphTopology,
        context: &'ctx inkwell::context::Context,
    ) {
        // HACK: recompiling all number inputs for now
        let dependents = topology.number_inputs().keys().cloned().collect();
        self.recompile_number_input_nodes(&dependents, topology, context);
    }

    fn modify_sound_input_node<F: FnMut(&mut dyn SoundInputNode<'ctx>)>(
        static_nodes: &mut [SharedProcessorNode<'ctx>],
        owner_id: SoundProcessorId,
        mut f: F,
    ) {
        let f_input = |_: &mut dyn SoundInputNode<'ctx>| true;
        let f_processor = |node: &mut dyn StateGraphNode<'ctx>| {
            if node.id() == owner_id {
                f(node.sound_input_node_mut());
                false
            } else {
                true
            }
        };
        visit_state_graph(f_input, f_processor, static_nodes);
    }

    fn modify_processor_node<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(
        static_nodes: &mut [SharedProcessorNode<'ctx>],
        processor_id: SoundProcessorId,
        mut f: F,
    ) {
        let f_input = |_: &mut dyn SoundInputNode<'ctx>| true;
        let f_processor = |node: &mut dyn StateGraphNode<'ctx>| {
            if node.id() == processor_id {
                f(node);
                false // stop recursing
            } else {
                true
            }
        };
        visit_state_graph(f_input, f_processor, static_nodes);
    }

    fn count_input_nodes(&mut self, input_id: SoundInputId) -> usize {
        // NOTE: this should ideally take &self, but I'm too lazy
        // to duplicate all of StateGraphVisitor to be non-mut
        let mut count = 0;

        let f_input = |node: &mut dyn SoundInputNode<'ctx>| {
            node.visit_inputs(&mut |id, _key, _target: &NodeTarget<'ctx>| {
                if id == input_id {
                    count += 1;
                }
            });
            true
        };
        let f_processor = |_: &mut dyn StateGraphNode<'ctx>| true;
        visit_state_graph(f_input, f_processor, &mut self.static_nodes);

        count
    }

    fn allocate_subgraph(
        static_nodes: &[SharedProcessorNode<'ctx>],
        processor_id: SoundProcessorId,
        topology: &SoundGraphTopology,
        context: &'ctx inkwell::context::Context,
    ) -> NodeTargetValue<'ctx> {
        // TODO: implement caching properly here
        // TODO: when caching, make sure to not cache nodes which are parameterized by different states
        let proc_data = topology.sound_processor(processor_id).unwrap();
        if proc_data.instance().is_static() {
            let shared_node = static_nodes
                .iter()
                .find(|n| n.id() == processor_id)
                .unwrap()
                .clone();
            NodeTargetValue::Shared(shared_node)
        } else {
            let mut node = proc_data.instance_arc().make_node(context);
            // TODO: do the equivalent of adding all soundinputs to the node right here.
            // Otherwise, they will be missing or inactive
            for input_id in proc_data.sound_inputs() {
                node.sound_input_node_mut().add_input(*input_id);
            }
            node.number_input_node_mut().visit_number_inputs_mut(
                &mut |n: &mut SoundNumberInputNode<'ctx>| {
                    n.recompile(topology, context);
                },
            );
            node.visit_sound_inputs_mut(
                &mut |input_id: SoundInputId,
                      _key_index: usize,
                      node: &mut NodeTarget<'ctx>,
                      timing: &mut InputTiming| {
                    debug_assert!(node.is_empty());
                    let input_data = topology.sound_input(input_id).unwrap();
                    let target = match input_data.target() {
                        Some(spid) => {
                            Self::allocate_subgraph(static_nodes, spid, topology, context)
                        }
                        None => NodeTargetValue::Empty,
                    };
                    node.set_target(target);
                    timing.require_reset();
                },
            );
            let unique_node = UniqueProcessorNode::new(node);
            NodeTargetValue::Unique(unique_node)
        }
    }

    fn recompile_number_input_nodes(
        &mut self,
        which_nodes: &HashSet<SoundNumberInputId>,
        topology: &SoundGraphTopology,
        context: &'ctx inkwell::context::Context,
    ) {
        let f_input = |_: &mut dyn SoundInputNode<'ctx>| true;
        let f_processor = |node: &mut dyn StateGraphNode<'ctx>| {
            node.visit_number_inputs_mut(&mut |ni: &mut SoundNumberInputNode<'ctx>| {
                if which_nodes.contains(&ni.id()) {
                    // TODO: do this not on the audio thread
                    ni.recompile(topology, context);
                }
            });
            true
        };
        visit_state_graph(f_input, f_processor, &mut self.static_nodes);
    }
}

struct StateGraphVisitor<'ctx, FInput, FProcessor>
where
    FInput: FnMut(&mut dyn SoundInputNode<'ctx>) -> bool,
    FProcessor: FnMut(&mut dyn StateGraphNode<'ctx>) -> bool,
{
    f_input: FInput,
    f_processor: FProcessor,
    visited_nodes: HashSet<*const ()>,
    phantom_data: PhantomData<&'ctx ()>,
}

impl<'ctx, FInput, FProcessor> StateGraphVisitor<'ctx, FInput, FProcessor>
where
    FInput: FnMut(&mut dyn SoundInputNode<'ctx>) -> bool,
    FProcessor: FnMut(&mut dyn StateGraphNode<'ctx>) -> bool,
{
    fn visit_node<'a>(&mut self, node: &'a mut dyn StateGraphNode<'ctx>) {
        let ptr = node.address();
        if self.visited_nodes.contains(&ptr) {
            return;
        }
        self.visited_nodes.insert(ptr);
        if (self.f_processor)(node) {
            if (self.f_input)(node.sound_input_node_mut()) {
                node.visit_sound_inputs_mut(self);
            }
        }
    }
}

impl<'ctx, FInput, FProcessor> SoundInputNodeVisitorMut<'ctx>
    for StateGraphVisitor<'ctx, FInput, FProcessor>
where
    FInput: FnMut(&mut dyn SoundInputNode<'ctx>) -> bool,
    FProcessor: FnMut(&mut dyn StateGraphNode<'ctx>) -> bool,
{
    fn visit_input(
        &mut self,
        _input_id: SoundInputId,
        _key_index: usize,
        target: &mut NodeTarget<'ctx>,
        timing: &mut InputTiming,
    ) {
        target.visit(|n: &mut dyn StateGraphNode<'ctx>| {
            self.visit_node(n);
        })
    }
}

fn visit_state_graph<'ctx, FInput, FProcessor>(
    f_input: FInput,
    f_processor: FProcessor,
    entry_points: &mut [SharedProcessorNode<'ctx>],
) where
    FInput: FnMut(&mut dyn SoundInputNode<'ctx>) -> bool,
    FProcessor: FnMut(&mut dyn StateGraphNode<'ctx>) -> bool,
{
    let mut visitor = StateGraphVisitor {
        f_input,
        f_processor,
        visited_nodes: HashSet::new(),
        phantom_data: PhantomData,
    };
    for node in entry_points {
        visitor.visit_node(node.borrow_data_mut().node_mut());
    }
}
