use std::collections::{HashMap, HashSet};

use hashstash::HashCacheProperty;

use crate::core::{
    sound::{
        argument::ProcessorArgumentLocation, expression::ProcessorExpressionLocation,
        soundgraph::SoundGraph, soundgraphvalidation::available_sound_expression_arguments,
    },
    stashing::StashingContext,
};

pub(crate) struct GraphProperties {
    // TODO: others?
    available_arguments:
        HashCacheProperty<HashMap<ProcessorExpressionLocation, HashSet<ProcessorArgumentLocation>>>,
}

impl GraphProperties {
    pub(crate) fn new(graph: &SoundGraph) -> GraphProperties {
        let mut props = GraphProperties {
            available_arguments: HashCacheProperty::new(),
        };
        props.refresh(graph);
        props
    }

    pub(crate) fn available_arguments(
        &self,
    ) -> &HashMap<ProcessorExpressionLocation, HashSet<ProcessorArgumentLocation>> {
        self.available_arguments.get_cached().unwrap()
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        self.available_arguments.refresh1_with_context(
            available_sound_expression_arguments,
            graph,
            &StashingContext::new_checking_recompilation(),
        );
    }
}
