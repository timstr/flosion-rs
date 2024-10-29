use eframe::egui;
use hashstash::{
    InplaceUnstasher, ObjectHash, Stash, Stashable, Stasher, UnstashError, UnstashableInplace,
};

use crate::core::{jit::cache::JitCache, sound::soundgraph::SoundGraph, stashing::StashingContext};

use super::{
    flosion_ui::Factories, graph_properties::GraphProperties, soundgraphuistate::SoundGraphUiState,
    stackedlayout::stackedlayout::StackedLayout,
};

pub(crate) struct AppState {
    /// The sound graph currently being used
    graph: SoundGraph,

    /// The state of the uis of all sound processors and their component uis
    ui_state: SoundGraphUiState,

    /// The on-screen layout of sound processors
    graph_layout: StackedLayout,

    properties: GraphProperties,

    previous_clean_revision: Option<ObjectHash>,
}

impl AppState {
    pub(crate) fn new() -> AppState {
        let graph = SoundGraph::new();

        let properties = GraphProperties::new(&graph);

        AppState {
            graph,
            ui_state: SoundGraphUiState::new(),
            graph_layout: StackedLayout::new(),
            properties,
            previous_clean_revision: None,
        }
    }

    pub(crate) fn graph(&self) -> &SoundGraph {
        &self.graph
    }

    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        jit_cache: &JitCache,
        stash: &Stash,
    ) {
        self.graph_layout.draw(
            ui,
            factories,
            &mut self.ui_state,
            &mut self.graph,
            &self.properties,
            jit_cache,
            stash,
        );

        self.ui_state.interact_and_draw(
            ui,
            factories,
            &mut self.graph,
            &self.properties,
            &mut self.graph_layout,
            stash,
        );
    }

    pub(crate) fn cleanup(&mut self, factories: &Factories) {
        self.properties.refresh(&self.graph);

        let current_revision = ObjectHash::from_stashable_and_context(
            &self.graph,
            &StashingContext::new_checking_recompilation(),
        );

        if self.previous_clean_revision != Some(current_revision) {
            self.graph_layout
                .regenerate(&self.graph, self.ui_state.positions());

            self.ui_state.cleanup(&self.graph, factories);

            self.previous_clean_revision = Some(current_revision);
        }

        self.ui_state.cleanup_frame_data();
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self) {
        assert_eq!(self.graph.validate(), Ok(()));
        self.ui_state.check_invariants(&self.graph);
        assert!(self.graph_layout.check_invariants(&self.graph));
    }
}

impl Stashable for AppState {
    fn stash(&self, stasher: &mut Stasher<()>) {
        // stash the graph
        stasher.object_with_context(&self.graph, &StashingContext::new_stashing_normally());

        // stash the ui state
        todo!();
        // stasher.object(&self.ui_state);

        // stash the layout
        todo!();
        // stasher.object(&self.graph_layout);

        // don't properties, they are derived from graph

        // also don't stash previous clean revision,
        // which is used for staying up to date
    }
}

impl UnstashableInplace for AppState {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        todo!()
    }
}
