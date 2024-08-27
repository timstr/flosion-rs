use crate::{
    core::{
        expression::expressiongraph::ExpressionGraph, graph::objectfactory::ObjectFactory,
        sound::soundgraph::SoundGraph,
    },
    ui_objects::all_objects::{all_expression_graph_objects, all_sound_graph_objects},
};
use eframe::{
    self,
    egui::{self},
};
use hashrevise::{Revisable, RevisionHash};

use super::{
    expressiongraphui::ExpressionGraphUi, graph_properties::GraphProperties,
    soundgraphui::SoundGraphUi, soundgraphuistate::SoundGraphUiState,
    stackedlayout::stackedlayout::StackedLayout, ui_factory::UiFactory,
};

/// Convenience struct for passing all the different factories together
pub(crate) struct Factories {
    sound_objects: ObjectFactory<SoundGraph>,
    expression_objects: ObjectFactory<ExpressionGraph>,
    sound_uis: UiFactory<SoundGraphUi>,
    expression_uis: UiFactory<ExpressionGraphUi>,
}

impl Factories {
    /// Creates a new set of factories pre-filled with all statically registered types
    pub(crate) fn new() -> Factories {
        let (object_factory, ui_factory) = all_sound_graph_objects();
        let (expression_object_factory, expression_ui_factory) = all_expression_graph_objects();

        Factories {
            sound_objects: object_factory,
            expression_objects: expression_object_factory,
            sound_uis: ui_factory,
            expression_uis: expression_ui_factory,
        }
    }

    pub(crate) fn sound_objects(&self) -> &ObjectFactory<SoundGraph> {
        &self.sound_objects
    }

    pub(crate) fn expression_objects(&self) -> &ObjectFactory<ExpressionGraph> {
        &self.expression_objects
    }

    pub(crate) fn sound_uis(&self) -> &UiFactory<SoundGraphUi> {
        &self.sound_uis
    }

    pub(crate) fn expression_uis(&self) -> &UiFactory<ExpressionGraphUi> {
        &self.expression_uis
    }
}

/// The very root of the GUI, which manages a SoundGraph instance,
/// responds to inputs, and draws the up-to-date ui via egui
pub struct FlosionApp {
    /// The sound graph currently being used
    graph: SoundGraph,

    /// Factories for instantiating sound and expression objects and their uis
    factories: Factories,

    /// The state of the uis of all sound processors and their component uis
    ui_state: SoundGraphUiState,

    /// The on-screen layout of sound processors
    graph_layout: StackedLayout,

    properties: GraphProperties,

    previous_clean_revision: Option<RevisionHash>,
}

impl FlosionApp {
    pub fn new(_cc: &eframe::CreationContext) -> FlosionApp {
        // TODO: learn about what CreationContext offers

        let graph = SoundGraph::new();

        let properties = GraphProperties::new(graph.topology());

        let mut app = FlosionApp {
            graph,
            factories: Factories::new(),
            ui_state: SoundGraphUiState::new(),
            graph_layout: StackedLayout::new(),
            properties,
            previous_clean_revision: None,
        };

        // Initialize all necessary ui state
        app.cleanup();

        #[cfg(debug_assertions)]
        app.check_invariants();

        app
    }

    fn interact_and_draw(&mut self, ui: &mut egui::Ui) {
        self.graph_layout.draw(
            ui,
            &self.factories,
            &mut self.ui_state,
            &mut self.graph,
            &self.properties,
        );

        self.ui_state.interact_and_draw(
            ui,
            &self.factories,
            &mut self.graph,
            &self.properties,
            &mut self.graph_layout,
        );
    }

    fn cleanup(&mut self) {
        let topo = self.graph.topology();

        self.properties.refresh(topo);

        let current_revision = topo.get_revision();

        if self.previous_clean_revision != Some(current_revision) {
            self.graph_layout
                .regenerate(topo, self.ui_state.positions());

            self.ui_state
                .cleanup_stale_graph_objects(topo, &self.factories);

            self.previous_clean_revision = Some(current_revision);
        }

        self.ui_state.cleanup_frame_data();
    }

    #[cfg(debug_assertions)]
    fn check_invariants(&self) {
        assert!(self.ui_state.check_invariants(self.graph.topology()));
        assert!(self.graph_layout.check_invariants(self.graph.topology()));
    }
}

impl eframe::App for FlosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            #[cfg(debug_assertions)]
            self.check_invariants();

            self.interact_and_draw(ui);

            self.graph.flush_updates();

            self.cleanup();

            #[cfg(debug_assertions)]
            self.check_invariants();
        });
    }
}
