use std::collections::HashSet;

use super::{
    numberinput::{NumberInputId, NumberInputOwner},
    numberinputnode::NumberInputNode,
    numbersource::NumberSourceId,
    soundgraphdata::{NumberInputData, NumberSourceData, SoundInputData, SoundProcessorData},
    soundgraphedit::SoundGraphEdit,
    soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId,
    soundinputnode::{SoundInputNode, SoundInputNodeVisitorMut},
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

pub struct StateGraph {
    static_nodes: Vec<SharedProcessorNode>,
    entry_points: Vec<SharedProcessorNode>,
}

impl StateGraph {
    //  first stab: always create a new state graph whenever anything changes
    // pub(super) fn new(topology: &SoundGraphTopology) -> StateGraph {
    //     let mut static_nodes: HashMap<SoundProcessorId, SharedProcessorNode> = HashMap::new();
    //     let cached_dynamic_nodes: HashMap<SoundProcessorId, SharedProcessorNode> = HashMap::new();

    //     // Add all static processors (without input targets filled in)
    //     for proc_data in topology.sound_processors().values() {
    //         if proc_data.instance().is_static() {
    //             let proc_node = proc_data.instance_arc().make_node();
    //             let node = SharedProcessorNode::new(proc_node);
    //             // NOTE: no target input for the node (yet)
    //             static_nodes.insert(proc_data.id(), node);
    //         }
    //     }
    //     // visit all static processors and fill their input targets
    //     let mut cache_all_nodes_visitor = CacheAllNodes {
    //         static_nodes: &static_nodes,
    //         cached_dynamic_nodes,
    //         topology,
    //     };
    //     for node in static_nodes.values() {
    //         node.visit_inputs(&mut cache_all_nodes_visitor);
    //     }

    //     let mut uncached_single_target_nodes_visitor = UncacheSingleTargetNodes { topology };
    //     for node in static_nodes.values() {
    //         node.visit_inputs(&mut uncached_single_target_nodes_visitor);
    //     }

    //     let mut entry_points: Vec<SharedProcessorNode> = Vec::new();
    //     for node in static_nodes.values() {
    //         if node.data.lock().num_target_inputs() == 0 {
    //             entry_points.push(node.clone());
    //         }
    //     }

    //     // At least one of the static nodes must be an entry point (e.g.
    //     // not connected to any inputs), otherwise there would be a cycle
    //     debug_assert!(static_nodes.len() == 0 || entry_points.len() > 0);

    //     StateGraph {
    //         static_nodes,
    //         entry_points,
    //     }
    // }

    pub(super) fn new() -> StateGraph {
        StateGraph {
            static_nodes: Vec::new(),
            entry_points: Vec::new(),
        }
    }

    pub(super) fn entry_points(&self) -> &[SharedProcessorNode] {
        &self.entry_points
    }

    pub(super) fn make_edit(&mut self, edit: SoundGraphEdit, topology: &SoundGraphTopology) {
        match edit {
            SoundGraphEdit::AddSoundProcessor(data) => self.add_sound_processor(data),
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
                self.connect_sound_input(siid, spid, topology)
            }
            SoundGraphEdit::DisconnectSoundInput(siid) => {
                self.disconnect_sound_input(siid, topology)
            }
            SoundGraphEdit::AddNumberSource(data) => self.add_number_source(data),
            SoundGraphEdit::RemoveNumberSource(nsid, _owner) => self.remove_number_source(nsid),
            SoundGraphEdit::AddNumberInput(data) => self.add_number_input(data, topology),
            SoundGraphEdit::RemoveNumberInput(niid, _owner) => {
                self.remove_number_input(niid, topology)
            }
            SoundGraphEdit::ConnectNumberInput(niid, nsid) => {
                self.connect_number_input(niid, nsid, topology)
            }
            SoundGraphEdit::DisconnectNumberInput(niid) => {
                self.disconnect_number_input(niid, topology)
            }
        }
    }

    fn add_sound_processor(&mut self, data: SoundProcessorData) {
        // if the processor is static, add it as an entry point
        // otherwise, since it isn't connected to anything
        // and thus won't be called, nothing needs doing
        if !data.instance().is_static() {
            return;
        }
        let shared_node = SharedProcessorNode::new(data.instance_arc().make_node());
        self.entry_points.push(shared_node.clone());

        self.static_nodes.push(shared_node);
    }

    fn remove_sound_processor(&mut self, processor_id: SoundProcessorId) {
        // NOTE: the processor is assumed to already be completely
        // disconnected. Dynamic processor nodes will thus have no
        // nodes allocated, and only static processors will have
        // allocated nodes.
        self.entry_points.retain(|n| n.id() != processor_id);
        self.static_nodes.retain(|n| n.id() != processor_id);
    }

    fn add_sound_input(&mut self, data: SoundInputData) {
        Self::modify_sound_input_node(&mut self.entry_points, data.owner(), |node| {
            node.add_input(data.id());
        });
    }

    fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        Self::modify_sound_input_node(&mut self.entry_points, owner, |node| {
            node.remove_input(input_id);
        });
    }

    fn add_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
    ) {
        Self::modify_sound_input_node(&mut self.entry_points, owner_id, |node| {
            node.add_key(input_id, index);
        });
    }

    fn remove_sound_input_key(
        &mut self,
        input_id: SoundInputId,
        index: usize,
        owner_id: SoundProcessorId,
    ) {
        Self::modify_sound_input_node(&mut self.entry_points, owner_id, |node| {
            node.remove_key(input_id, index);
        });
    }

    fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
        topology: &SoundGraphTopology,
    ) {
        let input_data = topology.sound_input(input_id).unwrap();
        Self::modify_sound_input_node(&mut self.entry_points, input_data.owner(), |node| {
            node.visit_inputs_mut(
                &mut |_siid: SoundInputId, _kidx: usize, tgt: &mut NodeTarget| {
                    debug_assert!(tgt.is_empty());
                    // TODO: make this context-aware so that it detects reused nodes in a synchronous
                    // group and caches them.
                    // For now, no caching...
                    tgt.set_target(Self::allocate_subgraph(
                        &self.static_nodes,
                        processor_id,
                        topology,
                    ));
                },
            )
        });
    }

    fn disconnect_sound_input(&mut self, input_id: SoundInputId, topology: &SoundGraphTopology) {
        let input_data = topology.sound_input(input_id).unwrap();
        Self::modify_sound_input_node(&mut self.entry_points, input_data.owner(), |node| {
            node.visit_inputs_mut(
                &mut |_siid: SoundInputId, _kidx: usize, tgt: &mut NodeTarget| {
                    debug_assert!(!tgt.is_empty());
                    tgt.set_target(NodeTargetValue::Empty);
                },
            )
        });
    }

    fn add_number_source(&mut self, _data: NumberSourceData) {
        // The number source is not connected to anything,
        // and so can't be evaluated and can't trigger a re-compile
    }

    fn remove_number_source(&mut self, _source_id: NumberSourceId) {
        // similar to add_number_source
    }

    fn add_number_input(&mut self, data: NumberInputData, topology: &SoundGraphTopology) {
        match data.owner() {
            NumberInputOwner::SoundProcessor(spid) => {
                // if the number input belongs to a sound processor,
                // add a number input node to the processor's nodes
                // but leave them empty - the new number input can't
                // be connected to anything yet.
                self.modify_processor_node(spid, |node| {
                    node.number_input_node_mut().add_input(data.id());
                    node.visit_number_inputs_mut(&mut |input: &mut NumberInputNode| {
                        if input.id() == data.id() {
                            input.recompile(topology);
                        }
                    });
                });
            }
            NumberInputOwner::NumberSource(nsid) => {
                // if the number input belongs to a number source,
                // find all number input nodes which indirectly draw
                // from that number source and recompile them, since
                // merely adding the input may lead to different
                // behaviour from a number source
                debug_assert!(topology.number_input(data.id()).is_some());
                debug_assert!(
                    topology.number_input(data.id()).unwrap().owner()
                        == NumberInputOwner::NumberSource(nsid)
                );
                let dependents = self.find_all_number_dependents(data.id(), topology);
                self.recompile_number_input_nodes(&dependents);
            }
        }
    }

    fn remove_number_input(&mut self, input_id: NumberInputId, topology: &SoundGraphTopology) {
        let data = topology.number_input(input_id).unwrap();
        match data.owner() {
            NumberInputOwner::SoundProcessor(spid) => {
                self.modify_processor_node(spid, |node| {
                    node.number_input_node_mut().remove_input(input_id);
                    node.visit_number_inputs_mut(&mut |input: &mut NumberInputNode| {
                        if input.id() == input_id {
                            input.recompile(topology);
                        }
                    });
                });
            }
            NumberInputOwner::NumberSource(nsid) => {
                // TODO: ??????????????
                // HACK: recompiling everything
                let dependents: HashSet<NumberInputId> =
                    topology.number_inputs().keys().cloned().collect();
                self.recompile_number_input_nodes(&dependents);
            }
        }
    }

    fn connect_number_input(
        &mut self,
        input_id: NumberInputId,
        source_id: NumberSourceId,
        topology: &SoundGraphTopology,
    ) {
        // TODO:
        // - For keeping number inputs up to date, knowledge about dependencies
        //   before AND AFTER a connection is made or broken is needed.
        // - For proper caching, knowledge about whether two sound processors
        //   in the same synchronous group are parameterized by different state
        //   sources is also needed
        // - conceptually, a sound processor can be cached for a pair of inputs
        //   whenever it is guaranteed to produce the same output for both inputs
        // - to determine whether two processors are parameterized identically,
        //   - for each number input of the processor (in the topology):
        //      - recursively visit all number sources that the input
        //        depends on
        //      - whenever an overload node is found, ?????
        let dependents = self.find_all_number_dependents(input_id, topology);
        self.recompile_number_input_nodes(&dependents);
    }

    fn disconnect_number_input(&mut self, _input_id: NumberInputId, topology: &SoundGraphTopology) {
        // TODO: recompile only those number inputs which actually changed
        // HACK: recompiling all number inputs for now
        let dependents = topology.number_inputs().keys().cloned().collect();
        self.recompile_number_input_nodes(&dependents);
    }

    fn modify_sound_input_node<F: FnMut(&mut dyn SoundInputNode)>(
        entry_points: &mut [SharedProcessorNode],
        owner_id: SoundProcessorId,
        f: F,
    ) {
        struct Visitor<FF: FnMut(&mut dyn SoundInputNode)> {
            f: FF,
            owner_id: SoundProcessorId,
        }

        impl<FF: FnMut(&mut dyn SoundInputNode)> Visitor<FF> {
            fn visit_node(&mut self, node: &mut dyn StateGraphNode) {
                if node.id() == self.owner_id {
                    (self.f)(node.sound_input_node_mut());
                } else {
                    node.visit_sound_inputs_mut(self);
                }
            }
        }

        impl<FF: FnMut(&mut dyn SoundInputNode)> SoundInputNodeVisitorMut for Visitor<FF> {
            fn visit_input(
                &mut self,
                _input_id: SoundInputId,
                _key_index: usize,
                target: &mut NodeTarget,
            ) {
                target.visit(|n| {
                    self.visit_node(n);
                })
            }
        }
        let mut visitor = Visitor { f, owner_id };
        for node in entry_points {
            visitor.visit_node(node.borrow_data_mut().node_mut());
        }
    }

    fn modify_processor_node<F: FnMut(&mut dyn StateGraphNode)>(
        &mut self,
        processor_id: SoundProcessorId,
        f: F,
    ) {
        struct Visitor<FF: FnMut(&mut dyn StateGraphNode)> {
            f: FF,
            processor_id: SoundProcessorId,
        }

        impl<FF: FnMut(&mut dyn StateGraphNode)> Visitor<FF> {
            fn visit_node(&mut self, node: &mut dyn StateGraphNode) {
                if node.id() == self.processor_id {
                    (self.f)(node);
                } else {
                    node.visit_sound_inputs_mut(self);
                }
            }
        }

        impl<FF: FnMut(&mut dyn StateGraphNode)> SoundInputNodeVisitorMut for Visitor<FF> {
            fn visit_input(
                &mut self,
                _input_id: SoundInputId,
                _key_index: usize,
                target: &mut NodeTarget,
            ) {
                target.visit(|n| {
                    self.visit_node(n);
                })
            }
        }
        let mut visitor = Visitor { f, processor_id };
        for node in &mut self.entry_points {
            visitor.visit_node(node.borrow_data_mut().node_mut());
        }
    }

    fn allocate_subgraph(
        static_nodes: &[SharedProcessorNode],
        processor_id: SoundProcessorId,
        topology: &SoundGraphTopology,
    ) -> NodeTargetValue {
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
            let mut node = proc_data.instance_arc().make_node();
            node.visit_sound_inputs_mut(
                &mut |input_id: SoundInputId, _key_index: usize, node: &mut NodeTarget| {
                    debug_assert!(node.is_empty());
                    let input_data = topology.sound_input(input_id).unwrap();
                    let target = match input_data.target() {
                        Some(spid) => Self::allocate_subgraph(static_nodes, spid, topology),
                        None => NodeTargetValue::Empty,
                    };
                    node.set_target(target);
                },
            );
            let unique_node = UniqueProcessorNode::new(node);
            NodeTargetValue::Unique(unique_node)
        }
    }

    fn find_all_number_dependents(
        &self,
        input_id: NumberInputId,
        topology: &SoundGraphTopology,
    ) -> HashSet<NumberInputId> {
        // TODO: consider caching this between edits

        fn visitor(
            niid: NumberInputId,
            topology: &SoundGraphTopology,
            dependents: &mut HashSet<NumberInputId>,
        ) {
            if dependents.contains(&niid) {
                return;
            }
            let input_data = topology.number_input(niid).unwrap();
            if let Some(tgt) = input_data.target() {
                let ns_data = topology.number_source(tgt).unwrap();
                for ns_input in ns_data.inputs() {
                    visitor(*ns_input, topology, dependents);
                }
            }
        }

        let mut dependents = HashSet::<NumberInputId>::new();

        for niid in topology.number_inputs().keys() {
            visitor(*niid, topology, &mut dependents);
        }

        dependents
    }

    fn recompile_number_input_nodes(&mut self, nodes: &HashSet<NumberInputId>) {
        todo!();
    }
}

// struct CacheAllNodes<'a> {
//     static_nodes: &'a HashMap<SoundProcessorId, SharedProcessorNode>,
//     cached_dynamic_nodes: HashMap<SoundProcessorId, SharedProcessorNode>,
//     topology: &'a SoundGraphTopology,
// }

// impl<'a> SoundInputNodeVisitorMut for CacheAllNodes<'a> {
//     fn visit_input(&mut self, input_id: SoundInputId, target: &mut NodeTarget) {
//         debug_assert!(target.is_empty());
//         let input_data = self.topology.sound_input(input_id).unwrap();
//         if let Some(processor_id) = input_data.target() {
//             let processor_data = self.topology.sound_processor(processor_id).unwrap();
//             if processor_data.instance().is_static() {
//                 let node = self.static_nodes.get(&processor_id).unwrap().clone();
//                 target.set_target(NodeTargetValue::Shared(node));
//             } else {
//                 let mut was_added = false;
//                 let make_default_node = || {
//                     was_added = true;
//                     SharedProcessorNode::new(processor_data.instance_arc().make_node())
//                 };

//                 let mut node = self
//                     .cached_dynamic_nodes
//                     .entry(processor_id)
//                     .or_insert_with(make_default_node)
//                     .clone();

//                 target.set_target(NodeTargetValue::Shared(node.clone()));

//                 if was_added {
//                     // TODO: this fails to account for nodes that can't be cached due to
//                     // their being parameterized by a distinct set of number sources.
//                     // The number sources drawn upon by the node **and its dependencies**
//                     // need to be compared to those along other destination inputs here.
//                     // Keep in mind that keyed input number sources are considered distinct
//                     // from one another.
//                     match input_data.options() {
//                         InputOptions::Synchronous => node.visit_inputs_mut(self),
//                         InputOptions::NonSynchronous => {
//                             let mut other_visitor = CacheAllNodes {
//                                 static_nodes: self.static_nodes,
//                                 cached_dynamic_nodes: HashMap::new(),
//                                 topology: self.topology,
//                             };
//                             node.visit_inputs_mut(&mut other_visitor);
//                         }
//                     }
//                 }

//                 node.borrow_data_mut().add_target_input(input_id);
//             }
//         }
//     }
// }

// struct UncacheSingleTargetNodes<'a> {
//     topology: &'a SoundGraphTopology,
// }

// impl<'a> SoundInputNodeVisitorMut for UncacheSingleTargetNodes<'a> {
//     fn visit_input(&mut self, _input_id: SoundInputId, target: &mut NodeTarget) {
//         match target.target_mut() {
//             NodeTargetValue::Unique(unique_node) => {
//                 unique_node.node_mut().visit_sound_inputs_mut(self);
//             }
//             NodeTargetValue::Shared(shared_node) => {
//                 // TODO: avoid visiting the same shared node twice
//                 if self
//                     .topology
//                     .sound_processor(shared_node.id())
//                     .unwrap()
//                     .instance()
//                     .is_static()
//                 {
//                     return;
//                 }
//                 shared_node
//                     .borrow_data_mut()
//                     .node_mut()
//                     .visit_sound_inputs_mut(self);

//                 let mut tmp_node = NodeTargetValue::Empty;
//                 std::mem::swap(target.target_mut(), &mut tmp_node);
//                 if let NodeTargetValue::Shared(s) = tmp_node {
//                     target.set_target(NodeTargetValue::Unique(s.into_unique_node().unwrap()));
//                 } else {
//                     // tmp_node stores the value of target.target at the start of this
//                     // match expression, thus it is guaranteed to be shared and this
//                     // case is unreachable.
//                     panic!();
//                 }
//             }
//             NodeTargetValue::Empty => (),
//         }
//     }
// }
