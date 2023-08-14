use crate::core::number::{numbergraph::NumberGraph, numbergraphtopology::NumberGraphTopology};

use super::{
    graph_ui::GraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{AnyNumberObjectUiData, NumberGraphUiState, NumberObjectUiStates},
};

pub struct NumberGraphUi {}

impl GraphUi for NumberGraphUi {
    type Graph = NumberGraph;

    type State = NumberGraphUiState;

    type Context<'a> = NumberGraphUiContext<'a>;

    type ObjectUiData = AnyNumberObjectUiData;
}

pub struct NumberGraphUiData {
    ui_state: NumberGraphUiState,
    object_ui_states: NumberObjectUiStates,
}

impl NumberGraphUiData {
    pub(super) fn new(topo: &NumberGraphTopology) -> NumberGraphUiData {
        NumberGraphUiData {
            ui_state: NumberGraphUiState::new(topo),
            object_ui_states: NumberObjectUiStates::new(),
        }
    }

    pub(super) fn ui_state(&self) -> &NumberGraphUiState {
        &self.ui_state
    }

    pub(super) fn object_ui_states(&self) -> &NumberObjectUiStates {
        &self.object_ui_states
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        self.ui_state.cleanup(topology);
        self.object_ui_states.cleanup(topology);
    }
}
