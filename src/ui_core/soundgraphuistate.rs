use crate::core::sound::soundgraphtopology::SoundGraphTopology;

use super::{
    expressiongraphuistate::ExpressionUiCollection, flosion_ui::Factories,
    soundgraphuinames::SoundGraphUiNames, soundobjectuistate::SoundObjectUiStates,
};

// TODO: rename to AppUiState
pub struct SoundGraphUiState {
    /// The ui information needed for all expression uis
    expression_uis: ExpressionUiCollection,

    /// The per-object ui information of all sound objects (for now, processor UIs)
    object_states: SoundObjectUiStates,

    /// The cached names of all objects in the ui
    names: SoundGraphUiNames,
}

impl SoundGraphUiState {
    pub(super) fn new() -> SoundGraphUiState {
        SoundGraphUiState {
            expression_uis: ExpressionUiCollection::new(),
            object_states: SoundObjectUiStates::new(),
            names: SoundGraphUiNames::new(),
        }
    }

    /// Remove any state associated with objects that are no longer present
    /// in the topology, and create new states for new objects
    pub(super) fn cleanup(&mut self, topo: &SoundGraphTopology, factories: &Factories) {
        self.object_states.cleanup(topo);

        self.expression_uis
            .cleanup(topo, factories.expression_uis());

        self.names.regenerate(topo);
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        self.object_states.check_invariants(topo)
    }

    pub(crate) fn names(&self) -> &SoundGraphUiNames {
        &self.names
    }

    pub(crate) fn names_mut(&mut self) -> &mut SoundGraphUiNames {
        &mut self.names
    }
}
