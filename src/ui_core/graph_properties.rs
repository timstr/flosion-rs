use std::collections::{HashMap, HashSet};

use hashrevise::RevisedProperty;

use crate::core::sound::{
    expression::SoundExpressionId, expressionargument::SoundExpressionArgumentId,
    soundgraph::SoundGraph, soundgraphvalidation::available_sound_expression_arguments,
};

pub(crate) struct GraphProperties {
    // TODO: others?
    available_arguments:
        RevisedProperty<HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>>,
}

impl GraphProperties {
    pub(crate) fn new(graph: &SoundGraph) -> GraphProperties {
        let mut props = GraphProperties {
            available_arguments: RevisedProperty::new(),
        };
        props.refresh(graph);
        props
    }

    pub(crate) fn available_arguments(
        &self,
    ) -> &HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>> {
        self.available_arguments.get_cached().unwrap()
    }

    pub(crate) fn refresh(&mut self, graph: &SoundGraph) {
        self.available_arguments
            .refresh1(available_sound_expression_arguments, graph);
    }
}
