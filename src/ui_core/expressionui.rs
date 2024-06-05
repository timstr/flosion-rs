use eframe::egui;

use crate::core::{
    expression::{
        expressiongraph::ExpressionGraph, expressiongraphtopology::ExpressionGraphTopology,
    },
    graph::objectfactory::ObjectFactory,
    sound::expression::SoundExpressionId,
};

use super::{
    expressiongraphui::ExpressionGraphUi,
    expressiongraphuicontext::{ExpressionGraphUiContext, OuterExpressionGraphUiContext},
    expressiongraphuistate::{ExpressionGraphUiState, ExpressionNodeObjectUiStates},
    expressionplot::{ExpressionPlot, PlotConfig},
    lexicallayout::lexicallayout::{LexicalLayout, LexicalLayoutFocus},
    ui_factory::UiFactory,
};

// TODO: add other presentations (e.g. plot, DAG maybe) and allow non-destructively switching between them
// TODO: rename
pub(super) struct ExpressionPresentation {
    lexical_layout: LexicalLayout,
    ui_states: ExpressionNodeObjectUiStates,
}

impl ExpressionPresentation {
    pub(super) fn new(
        topology: &ExpressionGraphTopology,
        factory: &UiFactory<ExpressionGraphUi>,
    ) -> ExpressionPresentation {
        let ui_states = ExpressionNodeObjectUiStates::generate(topology, factory);
        ExpressionPresentation {
            lexical_layout: LexicalLayout::generate(topology, &ui_states),
            ui_states,
        }
    }

    pub(super) fn cleanup(&mut self, topology: &ExpressionGraphTopology) {
        self.lexical_layout.cleanup(topology);
        self.ui_states.cleanup(topology);
    }

    pub(super) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        object_factory: &ObjectFactory<ExpressionGraph>,
        ui_factory: &UiFactory<ExpressionGraphUi>,
        outer_context: &mut OuterExpressionGraphUiContext,
    ) {
        self.lexical_layout.handle_keypress(
            ui,
            focus,
            object_factory,
            ui_factory,
            &mut self.ui_states,
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
        ui_state: &mut ExpressionGraphUiState,
        ctx: &ExpressionGraphUiContext,
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
                        .show(ui, ui_state, ctx, focus, outer_context);
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
