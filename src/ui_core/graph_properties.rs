use std::collections::{HashMap, HashSet};

use hashstash::HashCache;

use crate::core::sound::{
    expression::ProcessorExpressionLocation, expressionargument::SoundExpressionArgumentId,
    soundgraph::SoundGraph, soundgraphvalidation::available_sound_expression_arguments,
};

pub(crate) struct GraphProperties {
    // TODO: others?
    available_arguments:
        HashCache<HashMap<ProcessorExpressionLocation, HashSet<SoundExpressionArgumentId>>>,
}

impl GraphProperties {
    pub(crate) fn new(graph: &SoundGraph) -> GraphProperties {
        let mut props = GraphProperties {
            available_arguments: HashCache::new(),
        };
        props.refresh(graph);
        props
    }

    pub(crate) fn available_arguments(
        &self,
    ) -> &HashMap<ProcessorExpressionLocation, HashSet<SoundExpressionArgumentId>> {
        self.available_arguments.get_cached().unwrap()
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        self.available_arguments
            .refresh1(available_sound_expression_arguments, graph);
    }
}
