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
    pub(crate) fn new(topo: &SoundGraph) -> GraphProperties {
        let mut props = GraphProperties {
            available_arguments: RevisedProperty::new(),
        };
        props.refresh(topo);
        props
    }

    pub(crate) fn available_arguments(
        &self,
    ) -> &HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>> {
        self.available_arguments.get_cached().unwrap()
    }

    pub(crate) fn refresh(&mut self, topo: &SoundGraph) {
        self.available_arguments
            .refresh1(available_sound_expression_arguments, topo);
    }
}
