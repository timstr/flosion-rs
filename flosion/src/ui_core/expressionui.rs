use eframe::egui;

use crate::core::{expression::expressiongraph::ExpressionGraph, sound::soundgraph::SoundGraph};

use super::{
    expressiongraphuicontext::{ExpressionGraphUiContext, OuterExpressionGraphUiContext},
    expressiongraphuistate::ExpressionGraphUiState,
    expressionplot::{ExpressionPlot, PlotConfig},
    lexicallayout::lexicallayout::LexicalLayout,
};

pub(super) struct SoundExpressionUi {}

impl SoundExpressionUi {
    pub(super) fn new() -> SoundExpressionUi {
        SoundExpressionUi {}
    }

    pub(super) fn show(
        self,
        ui: &mut egui::Ui,
        ui_state: &mut ExpressionGraphUiState,
        ctx: &ExpressionGraphUiContext,
        layout: &mut LexicalLayout,
        expr_graph: &mut ExpressionGraph,
        outer_context: &OuterExpressionGraphUiContext,
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
                    match outer_context {
                        OuterExpressionGraphUiContext::ProcessorExpression(proc_expr_ctx) => {
                            layout.show(ui, ui_state, expr_graph, ctx, outer_context);

                            ExpressionPlot::new().show(
                                ui,
                                ctx.jit_cache(),
                                proc_expr_ctx.location(),
                                expr_graph,
                                proc_expr_ctx.mapping(),
                                *proc_expr_ctx.time_axis(),
                                plot_config,
                                proc_expr_ctx.sound_graph_names(),
                            );
                        }
                    }
                });
            })
            .inner;
    }
}
