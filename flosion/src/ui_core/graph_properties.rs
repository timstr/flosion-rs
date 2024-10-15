use std::collections::{HashMap, HashSet};

use hashstash::HashCacheProperty;

use crate::core::sound::{
    expression::ProcessorExpressionLocation, expressionargument::ArgumentLocation,
    soundgraph::SoundGraph, soundgraphvalidation::available_sound_expression_arguments,
};

pub(crate) struct GraphProperties {
    // TODO: others?
    available_arguments:
        HashCacheProperty<HashMap<ProcessorExpressionLocation, HashSet<ArgumentLocation>>>,
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
    ) -> &HashMap<ProcessorExpressionLocation, HashSet<ArgumentLocation>> {
        self.available_arguments.get_cached().unwrap()
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        self.available_arguments
            .refresh1(available_sound_expression_arguments, graph);
    }
}
