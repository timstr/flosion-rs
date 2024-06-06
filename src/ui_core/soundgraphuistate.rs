use std::rc::Rc;

use eframe::egui;

use crate::core::sound::{
    expression::SoundExpressionId, soundgraph::SoundGraph, soundgraphid::SoundObjectId,
    soundgraphtopology::SoundGraphTopology,
};

use super::{
    expressiongraphuicontext::{
        ExpressionGraphUiContext, OuterExpressionGraphUiContext, OuterProcessorExpressionContext,
    },
    expressiongraphuistate::ExpressionUiCollection,
    expressionplot::PlotConfig,
    expressionui::SoundExpressionUi,
    flosion_ui::Factories,
    graph_ui::GraphUiState,
    soundgraphui::SoundGraphUi,
    soundgraphuicontext::SoundGraphUiContext,
    soundgraphuinames::SoundGraphUiNames,
    soundobjectuistate::{AnySoundObjectUiData, SoundObjectUiStates},
    ui_factory::UiFactory,
};

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

    pub(crate) fn expression_uis_mut(&mut self) -> &mut ExpressionUiCollection {
        &mut self.expression_uis
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

    pub(crate) fn show_expression_graph_ui(
        &mut self,
        expression_id: SoundExpressionId,
        graph: &mut SoundGraph,
        ctx: &SoundGraphUiContext,
        plot_config: &PlotConfig,
        ui: &mut egui::Ui,
    ) {
        let parent_proc = graph.topology().expression(expression_id).unwrap().owner();
        let outer_ctx = OuterProcessorExpressionContext::new(
            expression_id,
            parent_proc,
            graph,
            &self.names,
            *ctx.time_axis(),
            ctx.available_arguments().get(&expression_id).unwrap(),
        );
        let mut outer_ctx = OuterExpressionGraphUiContext::ProcessorExpression(outer_ctx);
        let inner_ctx = ExpressionGraphUiContext::new(ctx.factories().expression_uis());

        let expr_ui_focus = None; // TODO

        let expr_ui = SoundExpressionUi::new(expression_id);

        let (expr_ui_state, expr_ui_layout) = self.expression_uis.get_mut(expression_id).unwrap();

        expr_ui.show(
            ui,
            expr_ui_state,
            &inner_ctx,
            expr_ui_layout,
            expr_ui_focus,
            &mut outer_ctx,
            plot_config,
        );
    }
}

impl GraphUiState for SoundGraphUiState {
    type GraphUi = SoundGraphUi;

    fn get_object_ui_data(&self, id: SoundObjectId) -> Rc<AnySoundObjectUiData> {
        self.object_states.get_object_data(id)
    }
}
