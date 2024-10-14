use std::collections::HashMap;

use crate::core::sound::{
    soundgraph::SoundGraph,
    soundprocessor::{AnySoundProcessor, SoundProcessorId},
};

use super::{
    stategraph::StateGraph,
    stategraphnode::{
        AnyCompiledProcessorData, SharedCompiledProcessor, SharedCompiledProcessorCache,
        UniqueCompiledProcessor,
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
    fn visit_unique_processor(&mut self, proc: &UniqueCompiledProcessor<'ctx>) -> bool {
        if !self.check_processor(proc.processor(), None) {
            return false;
        }
        self.visit_processor_sound_inputs(proc.processor())
    }

    /// Recursively inspect a compiled processor directly
    fn check_processor(
        &mut self,
        proc: &dyn AnyCompiledProcessorData<'ctx>,
        shared_data: Option<*const SharedCompiledProcessorCache<'ctx>>,
    ) -> bool {
        let Some(proc_data) = self.sound_graph.sound_processor(proc.id()) else {
            println!(
                "state_graph_matches_sound_graph: a sound processor was found which shouldn't exist"
            );
            return false;
        };

        if proc_data.is_static() {
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
        proc: &dyn AnyCompiledProcessorData,
        proc_data: &dyn AnySoundProcessor,
    ) -> bool {
        todo!()
    }

    /// Check the compiled expressions
    fn check_processor_expressions(
        &self,
        proc: &dyn AnyCompiledProcessorData,
        proc_data: &dyn AnySoundProcessor,
    ) -> bool {
        todo!()
    }

    /// Recursively visit the compiled sound inputs of a processor
    /// and all of its targets
    fn visit_processor_sound_inputs(&mut self, proc: &dyn AnyCompiledProcessorData<'ctx>) -> bool {
        todo!()
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
        if pd.is_static() {
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
