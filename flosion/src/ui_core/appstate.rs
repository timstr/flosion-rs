use eframe::egui::{self};
use hashstash::{
    InplaceUnstasher, ObjectHash, Stash, Stashable, Stasher, UnstashError, UnstashableInplace,
};

use crate::core::{jit::cache::JitCache, sound::soundgraph::SoundGraph, stashing::StashingContext};

use super::{
    factories::Factories, graph_properties::GraphProperties, history::SnapshotFlag,
    soundgraphuistate::SoundGraphUiState, stackedlayout::stackedlayout::StackedLayout,
    stashing::UiUnstashingContext,
};

pub(crate) struct AppState {
    /// The state of the uis of all sound processors and their component uis
    ui_state: SoundGraphUiState,

    /// The on-screen layout of sound processors
    graph_layout: StackedLayout,

    properties: GraphProperties,

    previous_clean_revision: Option<ObjectHash>,
}

impl AppState {
    pub(crate) fn new() -> AppState {
        AppState {
            ui_state: SoundGraphUiState::new(),
            graph_layout: StackedLayout::new(),
            properties: GraphProperties::new(),
            previous_clean_revision: None,
        }
    }

    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        graph: &mut SoundGraph,
        factories: &Factories,
        jit_cache: &JitCache,
        stash: &Stash,
        snapshot_flag: &SnapshotFlag,
    ) {
        self.graph_layout.draw(
            ui,
            factories,
            &mut self.ui_state,
            graph,
            &self.properties,
            jit_cache,
            stash,
            &snapshot_flag,
        );

        self.ui_state.interact_and_draw(
            ui,
            factories,
            graph,
            &self.properties,
            &mut self.graph_layout,
            stash,
            &snapshot_flag,
        );
    }

    pub(crate) fn cleanup(&mut self, graph: &SoundGraph, factories: &Factories) {
        self.properties.refresh(graph);

        let current_revision = ObjectHash::from_stashable_and_context(
            graph,
            StashingContext::new_checking_recompilation(),
        );

        if self.previous_clean_revision != Some(current_revision) {
            self.graph_layout
                .regenerate(graph, self.ui_state.positions());

            self.ui_state.cleanup(graph, factories);

            self.previous_clean_revision = Some(current_revision);
        }

        self.ui_state.cleanup_frame_data();
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, graph: &SoundGraph) {
        self.ui_state.check_invariants(graph);
        assert!(self.graph_layout.check_invariants(graph));
    }
}

impl Stashable for AppState {
    fn stash(&self, stasher: &mut Stasher<()>) {
        // stash the ui state
        stasher.object(&self.ui_state);

        // stash the layout
        stasher.object(&self.graph_layout);

        // don't stash properties, they are derived from graph

        // also don't stash previous clean revision,
        // which is used for staying up to date
    }
}

impl UnstashableInplace<UiUnstashingContext<'_>> for AppState {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UiUnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.ui_state)?;

        unstasher.object_inplace_with_context(&mut self.graph_layout, ())?;

        Ok(())
    }
}
