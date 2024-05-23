use eframe::egui;

use crate::core::{
    graph::objectfactory::ObjectFactory,
    number::{numbergraph::NumberGraph, numbergraphtopology::NumberGraphTopology},
    sound::soundnumberinput::SoundNumberInputId,
};

use super::{
    lexicallayout::lexicallayout::{LexicalLayout, LexicalLayoutFocus},
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::{NumberGraphUiContext, OuterNumberGraphUiContext},
    numbergraphuistate::{NumberGraphUiState, NumberObjectUiStates},
    numberinputplot::{NumberInputPlot, PlotConfig},
    ui_factory::UiFactory,
};

// TODO: add other presentations (e.g. plot, DAG maybe) and allow non-destructively switching between them
pub(super) struct SoundNumberInputPresentation {
    lexical_layout: LexicalLayout,
}

impl SoundNumberInputPresentation {
    pub(super) fn new(
        topology: &NumberGraphTopology,
        object_ui_states: &NumberObjectUiStates,
    ) -> SoundNumberInputPresentation {
        SoundNumberInputPresentation {
            lexical_layout: LexicalLayout::generate(topology, object_ui_states),
        }
    }

    pub(super) fn cleanup(
        &mut self,
        topology: &NumberGraphTopology,
        object_ui_states: &NumberObjectUiStates,
    ) {
        self.lexical_layout.cleanup(topology, object_ui_states);
    }

    pub(super) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut LexicalLayoutFocus,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        outer_context: &mut OuterNumberGraphUiContext,
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

pub(super) struct SoundNumberInputUi {
    _number_input_id: SoundNumberInputId,
}

impl SoundNumberInputUi {
    pub(super) fn new(id: SoundNumberInputId) -> SoundNumberInputUi {
        SoundNumberInputUi {
            _number_input_id: id,
        }
    }

    pub(super) fn show(
        self,
        ui: &mut egui::Ui,
        graph_state: &mut NumberGraphUiState,
        ctx: &mut NumberGraphUiContext,
        presentation: &mut SoundNumberInputPresentation,
        focus: Option<&mut LexicalLayoutFocus>,
        outer_context: &mut OuterNumberGraphUiContext,
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
                        OuterNumberGraphUiContext::SoundNumberInput(ctx) => {
                            NumberInputPlot::new().show(
                                ui,
                                ctx.jit_client(),
                                ctx.sound_graph()
                                    .topology()
                                    .number_input(ctx.sound_number_input_id())
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
