use std::collections::{HashMap, HashSet};

use crate::core::{
    sound::{
        soundgraphdata::{SoundInputBranchId, SoundProcessorData},
        soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId,
        soundnumberinput::SoundNumberInputId,
        soundprocessor::SoundProcessorId,
    },
    uniqueid::UniqueId,
};

use super::{
    soundnumberinputnode::SoundNumberInputNode,
    stategraph::StateGraph,
    stategraphnode::{
        NodeTargetValue, SharedProcessorNode, SharedProcessorNodeData, StateGraphNode,
        UniqueProcessorNode,
    },
};

struct Visitor<'a, 'ctx> {
    topology: &'a SoundGraphTopology,

    // All shared nodes visited so far, and the processors they correspond to.
    // Note that all static processor nodes are shared nodes, but dynamic
    // processor nodes can be shared as well.
    visited_shared_nodes: HashMap<*const SharedProcessorNodeData<'ctx>, SoundProcessorId>,

    // All static processors visited so far, along with the shared data that
    // they correspond to
    visited_static_processors: HashMap<SoundProcessorId, *const SharedProcessorNodeData<'ctx>>,
}

impl<'a, 'ctx> Visitor<'a, 'ctx> {
    fn visit_shared_processor_node(&mut self, node: &SharedProcessorNode<'ctx>) -> bool {
        let data = node.borrow_data();
        let data_ptr: *const SharedProcessorNodeData = &*data;

        if let Some(spid) = self.visited_shared_nodes.get(&data_ptr) {
            if *spid != node.id() {
                println!(
                    "state_graph_matches_topology: a single shared node exists with \
                    multiple processor ids"
                );
            }

            return true; // Already visited, presumably without finding errors
        }
        self.visited_shared_nodes.insert(data_ptr, node.id());

        if !self.check_processor(data.node(), Some(data_ptr)) {
            return false;
        }

        self.visit_processor_sound_inputs(data.node())
    }

    fn visit_unique_processor_node(&mut self, node: &UniqueProcessorNode<'ctx>) -> bool {
        if !self.check_processor(node.node(), None) {
            return false;
        }
        self.visit_processor_sound_inputs(node.node())
    }

    fn check_processor(
        &mut self,
        node: &dyn StateGraphNode<'ctx>,
        shared_data: Option<*const SharedProcessorNodeData<'ctx>>,
    ) -> bool {
        let Some(proc_data) = self.topology.sound_processors().get(&node.id()) else {
            println!(
                "state_graph_matches_topology: a sound processor was found which shouldn't exist"
            );
            return false;
        };

        if proc_data.instance().is_static() {
            let Some(shared_data_ptr) = shared_data else {
                println!(
                    "state_graph_matches_topology: found a unique node for a static processor \
                    instead of a shared node"
                );
                return false;
            };
            if let Some(other_ptr) = self.visited_static_processors.get(&node.id()) {
                if *other_ptr != shared_data_ptr {
                    println!(
                        "state_graph_matches_topology: multiple different shared nodes exist for \
                        the same static processor"
                    );
                    return false;
                }
            } else {
                self.visited_static_processors
                    .insert(proc_data.id(), shared_data_ptr);
            }
        }

        if !self.check_processor_sound_inputs(node, proc_data) {
            return false;
        }

        // NOTE: number sources have nothing to be checked here -
        // they are implemented in the state graph via compiled number
        // inputs elsewhere, which read from the context's states

        if !self.check_processor_number_inputs(node, proc_data) {
            return false;
        }

        true
    }

    fn check_processor_sound_inputs(
        &self,
        node: &dyn StateGraphNode,
        proc_data: &SoundProcessorData,
    ) -> bool {
        // Verify that all expected sound inputs are present
        {
            let mut remaining_input_nodes: HashSet<(SoundInputId, SoundInputBranchId)> =
                HashSet::new();
            let mut unexpected_input_nodes: HashSet<(SoundInputId, SoundInputBranchId)> =
                HashSet::new();
            for input_id in proc_data.sound_inputs() {
                let input_data = self.topology.sound_input(*input_id).unwrap();
                for bid in input_data.branches() {
                    remaining_input_nodes.insert((*input_id, *bid));
                }
            }

            for target in node.sound_input_node().targets() {
                if !remaining_input_nodes.remove(&(target.id(), target.branch_id())) {
                    unexpected_input_nodes.insert((target.id(), target.branch_id()));
                }
            }

            if !unexpected_input_nodes.is_empty() {
                println!(
                    "state_graph_matches_topology: sound processor {} \"{}\" has the following \
                    sound input nodes which shouldn't exist: {}",
                    node.id().value(),
                    proc_data.instance_arc().as_graph_object().get_type().name(),
                    comma_separated_list(unexpected_input_nodes.iter().map(|x| format!(
                        "input {} (branch={})",
                        x.0.value(),
                        x.1.value()
                    )))
                );
                return false;
            }
            if !remaining_input_nodes.is_empty() {
                println!(
                    "state_graph_matches_topology: sound processor {} \"{}\" is missing the \
                    following sound input nodes: {}",
                    node.id().value(),
                    proc_data.instance_arc().as_graph_object().get_type().name(),
                    comma_separated_list(remaining_input_nodes.iter().map(|x| format!(
                        "input {} (branch={})",
                        x.0.value(),
                        x.1.value()
                    )))
                );
                return false;
            }
        }

        // verify that the sound inputs have the expected targets
        {
            let mut all_good = true;
            for target in node.sound_input_node().targets() {
                let input_data = self.topology.sound_input(target.id()).unwrap();
                if target.target_id() != input_data.target() {
                    all_good = false;
                }
            }
            if !all_good {
                println!("state_graph_matches_topology: a sound input has the wrong target");
                return false;
            }
        }

        // TODO: verify that dynamic processors are being cached correctly

        // Nothing of number sources to check

        true
    }

    fn check_processor_number_inputs(
        &self,
        node: &dyn StateGraphNode,
        proc_data: &SoundProcessorData,
    ) -> bool {
        let mut remaining_inputs: HashSet<SoundNumberInputId> =
            proc_data.number_inputs().iter().cloned().collect();
        let mut unexpected_inputs: HashSet<SoundNumberInputId> = HashSet::new();

        node.visit_number_inputs(&mut |number_input_node: &SoundNumberInputNode| {
            if !remaining_inputs.remove(&number_input_node.id()) {
                unexpected_inputs.insert(number_input_node.id());
            }
        });

        let mut all_good = true;

        if !unexpected_inputs.is_empty() {
            println!(
                "state_graph_matches_topology: sound processor {} \"{}\" has the \
                following number input nodes which shouldn't exist: {}",
                node.id().value(),
                proc_data.instance_arc().as_graph_object().get_type().name(),
                comma_separated_list(unexpected_inputs.iter().map(|x| x.value().to_string()))
            );
            all_good = false;
        }
        if !remaining_inputs.is_empty() {
            println!(
                "state_graph_matches_topology: sound processor {} \"{}\" is missing the \
                following number input nodes: {}",
                node.id().value(),
                proc_data.instance_arc().as_graph_object().get_type().name(),
                comma_separated_list(remaining_inputs.iter().map(|x| x.value().to_string()))
            );
            all_good = false;
        }

        // TODO: once number input nodes are more fleshed out, verify that
        // they are up to date.

        all_good
    }

    fn visit_processor_sound_inputs(&mut self, node: &dyn StateGraphNode<'ctx>) -> bool {
        let mut all_good = true;

        // node.visit_sound_inputs(
        //     &mut |_siid: SoundInputId, _kidx: usize, target: &NodeTarget<'ctx>| {
        for target in node.sound_input_node().targets() {
            let good = match target.target() {
                NodeTargetValue::Unique(n) => self.visit_unique_processor_node(n),
                NodeTargetValue::Shared(n) => self.visit_shared_processor_node(n),
                NodeTargetValue::Empty => true,
            };
            if !good {
                all_good = false;
                break;
            }
        }

        all_good
    }
}

pub(crate) fn state_graph_matches_topology(
    state_graph: &StateGraph,
    topology: &SoundGraphTopology,
) -> bool {
    let mut visitor = Visitor {
        topology,
        visited_shared_nodes: HashMap::new(),
        visited_static_processors: HashMap::new(),
    };

    for node in state_graph.static_nodes() {
        if !visitor.visit_shared_processor_node(node) {
            return false;
        }
    }

    for static_node_id in topology.sound_processors().values().filter_map(|pd| {
        if pd.instance().is_static() {
            Some(pd.id())
        } else {
            None
        }
    }) {
        if visitor
            .visited_static_processors
            .remove(&static_node_id)
            .is_none()
        {
            println!("state_graph_matches_topology: a static sound processor node is missing");
            return false;
        }
    }

    if !visitor.visited_static_processors.is_empty() {
        println!(
            "state_graph_matches_topology: one or more static sound processor nodes were found \
            which shouldn't exist"
        );
        return false;
    }

    true
}

fn comma_separated_list<I: Iterator<Item = String>>(iter: I) -> String {
    let mut v = iter.collect::<Vec<String>>();
    v.sort();
    v.join(", ")
}
