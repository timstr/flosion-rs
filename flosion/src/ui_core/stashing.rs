use crate::core::sound::soundgraph::SoundGraph;

use super::factories::Factories;

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
