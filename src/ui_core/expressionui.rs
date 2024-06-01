use eframe::egui;

use crate::core::{
    expression::{
        expressiongraph::ExpressionGraph, expressiongraphtopology::ExpressionGraphTopology,
    },
    graph::objectfactory::ObjectFactory,
    sound::expression::SoundExpressionId,
};

use super::{
    lexicallayout::lexicallayout::{LexicalLayout, LexicalLayoutFocus},
    expressiongraphui::ExpressionGraphUi,
    expressiongraphuicontext::{ExpressionGraphUiContext, OuterExpressionGraphUiContext},
    expressiongraphuistate::{ExpressionGraphUiState, ExpressionNodeObjectUiStates},
    expressionplot::{ExpressionPlot, PlotConfig},
    ui_factory::UiFactory,
};

// TODO: add other presentations (e.g. plot, DAG maybe) and allow non-destructively switching between them
pub(super) struct ExpressionPresentation {
    lexical_layout: LexicalLayout,
}

impl ExpressionPresentation {
    pub(super) fn new(
        topology: &ExpressionGraphTopology,
        object_ui_states: &ExpressionNodeObjectUiStates,
    ) -> ExpressionPresentation {
        ExpressionPresentation {
            lexical_layout: LexicalLayout::generate(topology, object_ui_states),
        }
    }

    pub(super) fn cleanup(
        &mut self,
        topology: &ExpressionGraphTopology,
        object_ui_states: &ExpressionNodeObjectUiStates,
    ) {
        self.lexical_layout.cleanup(topology, object_ui_states);
    }

    pub(super) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        object_factory: &ObjectFactory<ExpressionGraph>,
        ui_factory: &UiFactory<ExpressionGraphUi>,
        object_ui_states: &mut ExpressionNodeObjectUiStates,
        outer_context: &mut OuterExpressionGraphUiContext,
    ) {
        self.lexical_layout.handle_keypress(
            ui,
            focus,
            object_factory,
            ui_factory,
            object_ui_states,
            outer_context,
        )
    }
}

pub(super) struct SoundExpressionUi {
    _expression_id: SoundExpressionId,
}

impl SoundExpressionUi {
    pub(super) fn new(id: SoundExpressionId) -> SoundExpressionUi {
        SoundExpressionUi { _expression_id: id }
    }

    pub(super) fn show(
        self,
        ui: &mut egui::Ui,
        graph_state: &mut ExpressionGraphUiState,
        ctx: &mut ExpressionGraphUiContext,
        presentation: &mut ExpressionPresentation,
        focus: Option<&mut LexicalLayoutFocus>,
        outer_context: &mut OuterExpressionGraphUiContext,
        plot_config: &PlotConfig,
    ) {
        // TODO: expandable/collapsible popup window with full layout
        let frame = egui::Frame::default()
            .fill(egui::Color32::BLACK)
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(64)))
            .inner_margin(egui::Margin::same(5.0));
        frame
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    presentation
                        .lexical_layout
                        .show(ui, graph_state, ctx, focus, outer_context);
                    match outer_context {
                        OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                            ExpressionPlot::new().show(
                                ui,
                                ctx.jit_client(),
                                ctx.sound_graph()
                                    .topology()
                                    .expression(ctx.expression_id())
                                    .unwrap(),
                                *ctx.time_axis(),
                                plot_config,
                            );
                        }
                    }
                });
            })
            .inner;
    }
}
