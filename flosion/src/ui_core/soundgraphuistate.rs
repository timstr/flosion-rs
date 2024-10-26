use eframe::egui;
use hashstash::Stash;

use crate::{
    core::{
        audiofileio::load_audio_file,
        sound::{
            expression::{ProcessorExpression, ProcessorExpressionLocation},
            soundgraph::SoundGraph,
            soundprocessor::{SoundProcessorId, SoundProcessorWithId},
        },
    },
    objects::audioclip::AudioClip,
};

use super::{
    expressiongraphuicontext::{ExpressionGraphUiContext, OuterProcessorExpressionContext},
    expressiongraphuistate::ExpressionUiCollection,
    expressionplot::PlotConfig,
    expressionui::SoundExpressionUi,
    flosion_ui::Factories,
    globalinteractions::GlobalInteractions,
    graph_properties::GraphProperties,
    soundgraphuicontext::SoundGraphUiContext,
    soundgraphuinames::SoundGraphUiNames,
    soundobjectpositions::SoundObjectPositions,
    soundobjectuistate::SoundObjectUiStates,
    stackedlayout::stackedlayout::StackedLayout,
};

pub struct SoundGraphUiState {
    /// The ui information needed for all expression uis
    expression_uis: ExpressionUiCollection,

    /// The per-object ui information of all sound objects (for now, processor UIs)
    object_states: SoundObjectUiStates,

    /// The cached names of all objects in the ui
    names: SoundGraphUiNames,

    /// The top-level user interactions with the sound graph UI,
    /// such as drag & drop, keyboard shortcuts, but not interactions
    /// within individual processor UIs
    interactions: GlobalInteractions,

    /// The positions of on-screen things that need tracking for later lookup
    positions: SoundObjectPositions,
}

impl SoundGraphUiState {
    pub(super) fn new() -> SoundGraphUiState {
        SoundGraphUiState {
            expression_uis: ExpressionUiCollection::new(),
            object_states: SoundObjectUiStates::new(),
            names: SoundGraphUiNames::new(),
            interactions: GlobalInteractions::new(),
            positions: SoundObjectPositions::new(),
        }
    }

    pub(crate) fn object_states(&self) -> &SoundObjectUiStates {
        &self.object_states
    }

    pub(crate) fn interactions_mut(&mut self) -> &mut GlobalInteractions {
        &mut self.interactions
    }

    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        graph: &mut SoundGraph,
        properties: &GraphProperties,
        layout: &mut StackedLayout,
        stash: &Stash,
    ) {
        let bg_response = ui.interact_bg(egui::Sense::click_and_drag());

        ui.with_layer_id(
            egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("foreground_interactions"),
            ),
            |ui| {
                self.interactions.interact_and_draw(
                    ui,
                    factories,
                    graph,
                    properties,
                    layout,
                    &mut self.object_states,
                    &mut self.positions,
                    &mut self.expression_uis,
                    &self.names,
                    bg_response,
                    stash,
                );
            },
        );

        let dropped_files = ui.input(|i| i.raw.dropped_files.clone());

        for dropped_file in dropped_files {
            let path = dropped_file.path.as_ref().unwrap();
            println!("Loading {}", path.display());
            if let Ok(buf) = load_audio_file(path) {
                let audioclip = SoundProcessorWithId::<AudioClip>::new_default();
                audioclip.set_data(buf);
                graph.add_sound_processor(Box::new(audioclip));
                println!("Loaded {}", path.display());
            } else {
                println!("Failed to load {}", path.display());
            }
        }
    }

    /// Remove any state associated with objects that are no longer present
    /// in the graph, and create new states for new objects
    pub(super) fn cleanup(&mut self, graph: &SoundGraph, factories: &Factories) {
        self.object_states.cleanup(graph);

        self.expression_uis
            .cleanup(graph, factories.expression_uis());

        self.names.regenerate(graph);
        self.interactions.cleanup(graph);
    }

    pub(super) fn cleanup_frame_data(&mut self) {
        self.positions.clear();
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, graph: &SoundGraph) {
        self.object_states.check_invariants(graph);
        self.names.check_invariants(graph);
    }

    pub(crate) fn names(&self) -> &SoundGraphUiNames {
        &self.names
    }

    pub(crate) fn names_mut(&mut self) -> &mut SoundGraphUiNames {
        &mut self.names
    }

    pub(crate) fn positions(&self) -> &SoundObjectPositions {
        &self.positions
    }

    pub(crate) fn positions_mut(&mut self) -> &mut SoundObjectPositions {
        &mut self.positions
    }

    pub(crate) fn show_expression_graph_ui(
        &mut self,
        processor_id: SoundProcessorId,
        expr: &mut ProcessorExpression,
        ctx: &SoundGraphUiContext,
        plot_config: &PlotConfig,
        ui: &mut egui::Ui,
    ) {
        let location = ProcessorExpressionLocation::new(processor_id, expr.id());

        let (mapping, expr_graph) = expr.parts_mut();

        let outer_ctx = OuterProcessorExpressionContext::new(
            location,
            mapping,
            &self.names,
            *ctx.time_axis(),
            ctx.properties()
                .available_arguments()
                .get(&location)
                .unwrap(),
        );
        let inner_ctx = ExpressionGraphUiContext::new(
            ctx.factories().expression_objects(),
            ctx.factories().expression_uis(),
            ctx.jit_cache(),
            ctx.stash(),
        );

        let expr_ui = SoundExpressionUi::new();

        let (expr_ui_state, expr_ui_layout) = self.expression_uis.get_mut(location).unwrap();

        expr_ui.show(
            ui,
            expr_ui_state,
            &inner_ctx,
            expr_ui_layout,
            expr_graph,
            &outer_ctx.into(),
            plot_config,
        );
    }
}
