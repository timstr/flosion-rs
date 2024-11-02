use crate::core::{expression::expressiongraph::ExpressionGraph, sound::soundgraph::SoundGraph};

use super::{expressionobjectui::ExpressionObjectUiFactory, factories::Factories};

#[derive(Copy, Clone)]
pub struct UiUnstashingContext<'a> {
    // needed to create new object UI instances
    factories: &'a Factories,

    // needed by UIs to create UI state from existing objects
    sound_graph: &'a SoundGraph,
}

impl<'a> UiUnstashingContext<'a> {
    pub(crate) fn new(
        factories: &'a Factories,
        sound_graph: &'a SoundGraph,
    ) -> UiUnstashingContext<'a> {
        UiUnstashingContext {
            factories,
            sound_graph,
        }
    }

    pub(crate) fn factories(&self) -> &'a Factories {
        self.factories
    }

    pub(crate) fn sound_graph(&self) -> &'a SoundGraph {
        self.sound_graph
    }
}

#[derive(Copy, Clone)]
pub struct ExpressionUiUnstashingContext<'a> {
    factory: &'a ExpressionObjectUiFactory,
    expression_graph: &'a ExpressionGraph,
}

impl<'a> ExpressionUiUnstashingContext<'a> {
    pub(crate) fn new(
        factory: &'a ExpressionObjectUiFactory,
        expression_graph: &'a ExpressionGraph,
    ) -> ExpressionUiUnstashingContext<'a> {
        ExpressionUiUnstashingContext {
            factory,
            expression_graph,
        }
    }

    pub(crate) fn factory(&self) -> &'a ExpressionObjectUiFactory {
        self.factory
    }

    pub(crate) fn expression_graph(&self) -> &'a ExpressionGraph {
        self.expression_graph
    }
}
