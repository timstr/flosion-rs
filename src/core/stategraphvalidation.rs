use std::collections::HashSet;

use crate::core::stategraphnode::NodeTargetValue;

use super::{
    numberinput::NumberInputId,
    numberinputnode::NumberInputNode,
    soundgraphdata::SoundProcessorData,
    soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    stategraph::StateGraph,
    stategraphnode::{
        NodeTarget, SharedProcessorNode, SharedProcessorNodeData, StateGraphNode,
        UniqueProcessorNode,
    },
};

struct Visitor<'a> {
    topology: &'a SoundGraphTopology,
    visited_shared_nodes: HashSet<*const SharedProcessorNodeData>,
    visited_static_processors: HashSet<SoundProcessorId>,
}

impl<'a> Visitor<'a> {
    fn visit_shared_processor_node(&mut self, node: &SharedProcessorNode) -> bool {
        let data = node.borrow_data();
        let data_ptr: *const SharedProcessorNodeData = &*data;
        if self.visited_shared_nodes.contains(&data_ptr) {
            return true; // Already visited, presumably without finding errors
        }
        self.visited_shared_nodes.insert(data_ptr);

        if !self.check_processor(data.node()) {
            return false;
        }

        self.visit_processor_sound_inputs(data.node())
    }

    fn visit_unique_processor_node(&mut self, node: &UniqueProcessorNode) -> bool {
        if !self.check_processor(node.node()) {
            return false;
        }
        self.visit_processor_sound_inputs(node.node())
    }

    fn check_processor(&mut self, node: &dyn StateGraphNode) -> bool {
        let proc_data = match self.topology.sound_processors().get(&node.id()) {
            Some(p) => p,
            None => {
                println!("state_graph_matches_topology: a sound processor was found which shouldn't exist");
                return false;
            }
        };

        if proc_data.instance().is_static() {
            self.visited_static_processors.insert(proc_data.id());
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
            let mut remaining_input_nodes: HashSet<(SoundInputId, usize)> = HashSet::new();
            for input_id in proc_data.sound_inputs() {
                let input_data = self.topology.sound_input(*input_id).unwrap();
                for k in 0..input_data.num_keys() {
                    remaining_input_nodes.insert((*input_id, k));
                }
            }
            let mut all_good = true;

            node.visit_sound_inputs(&mut |siid: SoundInputId, kidx: usize, _tgt: &NodeTarget| {
                if !remaining_input_nodes.remove(&(siid, kidx)) {
                    all_good = false;
                }
            });

            if !all_good {
                println!("state_graph_matches_topology: a sound processor node has a sound input which shouldn't exist");
                return false;
            }
            if !remaining_input_nodes.is_empty() {
                println!("state_graph_matches_topology: a sound processor node is missing one or more sound inputs");
                return false;
            }
        }

        // verify that the sound inputs have the expected targets
        {
            let mut all_good = true;
            node.visit_sound_inputs(
                &mut |siid: SoundInputId, _kidx: usize, target: &NodeTarget| {
                    let input_data = self.topology.sound_input(siid).unwrap();
                    if target.id() != input_data.target() {
                        all_good = false;
                    }
                },
            );
            if !all_good {
                println!("state_graph_matches_topology: a sound input has the wrong target");
                return false;
            }
        }

        // TODO: verify that processors are being cached correctly

        // Nothing of number sources to check

        true
    }

    fn check_processor_number_inputs(
        &self,
        node: &dyn StateGraphNode,
        proc_data: &SoundProcessorData,
    ) -> bool {
        let mut remaining_inputs: HashSet<NumberInputId> =
            proc_data.number_inputs().iter().cloned().collect();
        let mut all_good = true;

        node.visit_number_inputs(&mut |number_input_node: &NumberInputNode| {
            if !remaining_inputs.remove(&number_input_node.id()) {
                all_good = false;
            }
        });

        if !all_good {
            println!("state_graph_matches_topology: a sound processor has a number input which shouldn't exist");
            return false;
        }
        if !remaining_inputs.is_empty() {
            println!("state_graph_matches_topology: a sound processor is missing one or more number inputs");
            return false;
        }

        // TODO: once number input nodes are more fleshed out, verify that
        // they are up to date.

        true
    }

    fn visit_processor_sound_inputs(&mut self, node: &dyn StateGraphNode) -> bool {
        let mut all_good = true;

        node.visit_sound_inputs(
            &mut |_siid: SoundInputId, _kidx: usize, target: &NodeTarget| {
                if !all_good {
                    return;
                }
                let good = match target.target() {
                    NodeTargetValue::Unique(n) => self.visit_unique_processor_node(n),
                    NodeTargetValue::Shared(n) => self.visit_shared_processor_node(n),
                    NodeTargetValue::Empty => true,
                };
                if !good {
                    all_good = false;
                }
            },
        );

        all_good
    }
}

pub(super) fn state_graph_matches_topology(
    state_graph: &StateGraph,
    topology: &SoundGraphTopology,
) -> bool {
    let mut visitor = Visitor {
        topology,
        visited_shared_nodes: HashSet::new(),
        visited_static_processors: HashSet::new(),
    };

    for ep in state_graph.entry_points() {
        if !visitor.visit_shared_processor_node(ep) {
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
        if !visitor.visited_static_processors.remove(&static_node_id) {
            println!("state_graph_matches_topology: a static sound processor node is missing");
            return false;
        }
    }

    if !visitor.visited_static_processors.is_empty() {
        println!("state_graph_matches_topology: one or more static sound processor nodes were found which shouldn't exist");
        return false;
    }

    true
}
