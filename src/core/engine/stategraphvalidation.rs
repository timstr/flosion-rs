use std::collections::{HashMap, HashSet};

use crate::core::sound::{
    expression::SoundExpressionId,
    soundgraph::SoundGraph,
    soundgraphdata::{SoundInputBranchId, SoundProcessorData},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::{
    compiledexpression::CompiledExpression,
    stategraph::StateGraph,
    stategraphnode::{
        CompiledSoundProcessor, SharedCompiledProcessor, SharedCompiledProcessorCache,
        StateGraphNodeValue, UniqueCompiledSoundProcessor,
    },
};

/// Helper struct for comparing a StateGraph to a SoundGraph instance
struct Visitor<'a, 'ctx> {
    /// The sound graph being compared against
    sound_graph: &'a SoundGraph,

    /// All shared compiled processors visited so far, and the processors they correspond to.
    /// Note that all static processor nodes are shared nodes, but dynamic
    /// processor nodes can be shared as well.
    visited_shared_processors: HashMap<*const SharedCompiledProcessorCache<'ctx>, SoundProcessorId>,

    /// All static processors visited so far, along with the shared data that
    /// they correspond to
    visited_static_processors: HashMap<SoundProcessorId, *const SharedCompiledProcessorCache<'ctx>>,
}

impl<'a, 'ctx> Visitor<'a, 'ctx> {
    /// Recursively inspect a shared compiled processor and test whether it matches
    /// the sound graph.
    fn visit_shared_processor(&mut self, proc: &SharedCompiledProcessor<'ctx>) -> bool {
        let data = proc.borrow_cache();
        let data_ptr: *const SharedCompiledProcessorCache = &*data;

        if let Some(spid) = self.visited_shared_processors.get(&data_ptr) {
            if *spid != proc.id() {
                println!(
                    "state_graph_matches_sound_graph: a single shared compiled processor exists with \
                    multiple processor ids"
                );
                return false;
            }

            return true; // Already visited, presumably without finding errors
        }
        self.visited_shared_processors.insert(data_ptr, proc.id());

        if !self.check_processor(data.processor(), Some(data_ptr)) {
            return false;
        }

        self.visit_processor_sound_inputs(data.processor())
    }

    /// Recursively inspect a unique compiled processor and test whether it matches
    /// the sound graph
    fn visit_unique_processor(&mut self, proc: &UniqueCompiledSoundProcessor<'ctx>) -> bool {
        if !self.check_processor(proc.processor(), None) {
            return false;
        }
        self.visit_processor_sound_inputs(proc.processor())
    }

    /// Recursively inspect a compiled processor directly
    fn check_processor(
        &mut self,
        proc: &dyn CompiledSoundProcessor<'ctx>,
        shared_data: Option<*const SharedCompiledProcessorCache<'ctx>>,
    ) -> bool {
        let Some(proc_data) = self.sound_graph.sound_processors().get(&proc.id()) else {
            println!(
                "state_graph_matches_sound_graph: a sound processor was found which shouldn't exist"
            );
            return false;
        };

        if proc_data.instance().is_static() {
            let Some(shared_data_ptr) = shared_data else {
                println!(
                    "state_graph_matches_sound_graph: found a unique node for a static processor \
                    instead of a shared node"
                );
                return false;
            };
            if let Some(other_ptr) = self.visited_static_processors.get(&proc.id()) {
                if *other_ptr != shared_data_ptr {
                    println!(
                        "state_graph_matches_sound_graph: multiple different shared nodes exist for \
                        the same static processor"
                    );
                    return false;
                }
            } else {
                self.visited_static_processors
                    .insert(proc_data.id(), shared_data_ptr);
            }
        }

        if !self.check_processor_sound_inputs(proc, proc_data) {
            return false;
        }

        // NOTE: expression arguments have nothing to be checked here -
        // they are implemented in the state graph via compiled expressions
        // elsewhere, which read from the context's states

        if !self.check_processor_expressions(proc, proc_data) {
            return false;
        }

        true
    }

    /// Check the compiled sound inputs
    fn check_processor_sound_inputs(
        &self,
        proc: &dyn CompiledSoundProcessor,
        proc_data: &SoundProcessorData,
    ) -> bool {
        // Verify that all expected sound inputs are present
        {
            let mut remaining_input_branches: HashSet<(SoundInputId, SoundInputBranchId)> =
                HashSet::new();
            let mut unexpected_input_branches: HashSet<(SoundInputId, SoundInputBranchId)> =
                HashSet::new();
            for input_id in proc_data.sound_inputs() {
                let input_data = self.sound_graph.sound_input(*input_id).unwrap();
                for bid in input_data.branches() {
                    remaining_input_branches.insert((*input_id, *bid));
                }
            }

            for target in proc.sound_input().targets() {
                if !remaining_input_branches.remove(&(target.id(), target.branch_id())) {
                    unexpected_input_branches.insert((target.id(), target.branch_id()));
                }
            }

            if !unexpected_input_branches.is_empty() {
                println!(
                    "state_graph_matches_sound_graph: sound processor {}  has the following \
                    sound input branches which shouldn't exist: {}",
                    proc_data.friendly_name(),
                    comma_separated_list(unexpected_input_branches.iter().map(|x| format!(
                        "input {} (branch={})",
                        x.0.value(),
                        x.1.value()
                    )))
                );
                return false;
            }
            if !remaining_input_branches.is_empty() {
                println!(
                    "state_graph_matches_sound_graph: sound processor {} is missing the \
                    following sound input branches: {}",
                    proc_data.friendly_name(),
                    comma_separated_list(remaining_input_branches.iter().map(|x| format!(
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
            for target in proc.sound_input().targets() {
                let input_data = self.sound_graph.sound_input(target.id()).unwrap();
                if target.target_id() != input_data.target() {
                    all_good = false;
                }
            }
            if !all_good {
                println!("state_graph_matches_sound_graph: a sound input has the wrong target");
                return false;
            }
        }

        // TODO: verify that dynamic processors are being cached correctly

        // Nothing of expression arguments to check

        true
    }

    /// Check the compiled expressions
    fn check_processor_expressions(
        &self,
        proc: &dyn CompiledSoundProcessor,
        proc_data: &SoundProcessorData,
    ) -> bool {
        let mut remaining_inputs: HashSet<SoundExpressionId> =
            proc_data.expressions().iter().cloned().collect();
        let mut unexpected_inputs: HashSet<SoundExpressionId> = HashSet::new();

        proc.expressions().visit(&mut |expr: &CompiledExpression| {
            if !remaining_inputs.remove(&expr.id()) {
                unexpected_inputs.insert(expr.id());
            }
        });

        let mut all_good = true;

        if !unexpected_inputs.is_empty() {
            println!(
                "state_graph_matches_sound_graph: sound processor {} has the \
                following compiled expressions which shouldn't exist: {}",
                proc_data.friendly_name(),
                comma_separated_list(unexpected_inputs.iter().map(|x| x.value().to_string()))
            );
            all_good = false;
        }
        if !remaining_inputs.is_empty() {
            println!(
                "state_graph_matches_sound_graph: sound processor {} is missing the \
                following compiled expressions: {}",
                proc_data.friendly_name(),
                comma_separated_list(remaining_inputs.iter().map(|x| x.value().to_string()))
            );
            all_good = false;
        }

        // TODO: verify that compiled expressions are up to date.

        all_good
    }

    /// Recursively visit the compiled sound inputs of a processor
    /// and all of its targets
    fn visit_processor_sound_inputs(&mut self, proc: &dyn CompiledSoundProcessor<'ctx>) -> bool {
        let mut all_good = true;

        for target in proc.sound_input().targets() {
            let good = match target.target() {
                StateGraphNodeValue::Unique(n) => self.visit_unique_processor(n),
                StateGraphNodeValue::Shared(n) => self.visit_shared_processor(n),
                StateGraphNodeValue::Empty => true,
            };
            if !good {
                all_good = false;
                break;
            }
        }

        all_good
    }
}

/// Checks whether the given state graph accurately models the given sound graph.
pub(crate) fn state_graph_matches_sound_graph(
    state_graph: &StateGraph,
    sound_graph: &SoundGraph,
) -> bool {
    let mut visitor = Visitor {
        sound_graph,
        visited_shared_processors: HashMap::new(),
        visited_static_processors: HashMap::new(),
    };

    for proc in state_graph.static_processors() {
        if !visitor.visit_shared_processor(proc) {
            return false;
        }
    }

    for static_proc_id in sound_graph.sound_processors().values().filter_map(|pd| {
        if pd.instance().is_static() {
            Some(pd.id())
        } else {
            None
        }
    }) {
        if visitor
            .visited_static_processors
            .remove(&static_proc_id)
            .is_none()
        {
            println!("state_graph_matches_sound_graph: a static compiled processor is missing");
            return false;
        }
    }

    if !visitor.visited_static_processors.is_empty() {
        println!(
            "state_graph_matches_sound_graph: one or more static compiled processors were found \
            which shouldn't exist"
        );
        return false;
    }

    true
}

/// Helper function for pretty-printing a sequence of strings
fn comma_separated_list<I: Iterator<Item = String>>(iter: I) -> String {
    let mut v = iter.collect::<Vec<String>>();
    v.sort();
    v.join(", ")
}
