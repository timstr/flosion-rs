use eframe::egui;

use crate::core::sound::{expression::SoundExpressionId, soundgraph::SoundGraph};

use super::{
    expressiongraphuicontext::{ExpressionGraphUiContext, OuterExpressionGraphUiContext},
    expressiongraphuistate::ExpressionGraphUiState,
    expressionplot::{ExpressionPlot, PlotConfig},
    lexicallayout::lexicallayout::LexicalLayout,
};

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
        ui_state: &mut ExpressionGraphUiState,
        ctx: &ExpressionGraphUiContext,
        layout: &mut LexicalLayout,
        sound_graph: &mut SoundGraph,
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
                    layout.show(ui, ui_state, sound_graph, ctx, outer_context);
                    match outer_context {
                        OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                            ExpressionPlot::new().show(
                                ui,
                                sound_graph.jit_client(),
                                sound_graph
                                    .topology()
                                    .expression(ctx.expression_id())
                                    .unwrap(),
                                *ctx.time_axis(),
                                plot_config,
                                ctx.sound_graph_names(),
                            );
                        }
                    }
                });
            })
            .inner;
    }
}
