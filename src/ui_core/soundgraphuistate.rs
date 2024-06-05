use std::rc::Rc;

use crate::core::sound::{soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology};

use super::{
    expressiongraphuistate::ExpressionUiCollection,
    flosion_ui::Factories,
    graph_ui::GraphUiState,
    soundgraphui::SoundGraphUi,
    soundgraphuinames::SoundGraphUiNames,
    soundobjectuistate::{AnySoundObjectUiData, SoundObjectUiStates},
    ui_factory::UiFactory,
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

    pub(crate) fn object_states(&self) -> &SoundObjectUiStates {
        &self.object_states
    }

    pub(super) fn create_state_for(
        &mut self,
        id: SoundObjectId,
        topo: &SoundGraphTopology,
        factory: &UiFactory<SoundGraphUi>,
    ) {
        let object_handle = topo.graph_object(id).unwrap();
        let state = factory.create_default_state(&object_handle);
        self.object_states.set_object_data(id, state);
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

impl GraphUiState for SoundGraphUiState {
    type GraphUi = SoundGraphUi;

    fn get_object_ui_data(&self, id: SoundObjectId) -> Rc<AnySoundObjectUiData> {
        self.object_states.get_object_data(id)
    }
}
