use eframe::egui;

use crate::core::{
    graph::objectfactory::ObjectFactory,
    number::{
        context::MockNumberContext,
        numbergraph::{NumberGraph, NumberGraphInputId},
        numbergraphtopology::NumberGraphTopology,
    },
    sound::soundnumberinput::SoundNumberInputId,
};

use super::{
    lexicallayout::lexicallayout::{LexicalLayout, LexicalLayoutFocus},
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::{NumberGraphUiContext, OuterNumberGraphUiContext},
    numbergraphuistate::{NumberGraphUiState, NumberObjectUiStates},
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

    pub(super) fn lexical_layout(&self) -> &LexicalLayout {
        &self.lexical_layout
    }

    pub(super) fn lexical_layout_mut(&mut self) -> &mut LexicalLayout {
        &mut self.lexical_layout
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

pub(super) struct SpatialGraphInputReference {
    input_id: NumberGraphInputId,
    location: egui::Pos2,
}

impl SpatialGraphInputReference {
    pub(super) fn new(
        input_id: NumberGraphInputId,
        location: egui::Pos2,
    ) -> SpatialGraphInputReference {
        SpatialGraphInputReference { input_id, location }
    }

    pub(super) fn input_id(&self) -> NumberGraphInputId {
        self.input_id
    }

    pub(super) fn location(&self) -> egui::Pos2 {
        self.location
    }

    pub(super) fn location_mut(&mut self) -> &mut egui::Pos2 {
        &mut self.location
    }
}

pub(super) struct SoundNumberInputUi {
    number_input_id: SoundNumberInputId,
}

impl SoundNumberInputUi {
    pub(super) fn new(id: SoundNumberInputId) -> SoundNumberInputUi {
        SoundNumberInputUi {
            number_input_id: id,
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
    ) -> Vec<SpatialGraphInputReference> {
        // TODO: expandable/collapsible popup window with full layout
        let frame = egui::Frame::default()
            .fill(egui::Color32::BLACK)
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(64)))
            .inner_margin(egui::Margin::same(5.0));
        let rev = match outer_context {
            OuterNumberGraphUiContext::SoundNumberInput(snictx) => snictx
                .sound_graph()
                .topology()
                .number_input(snictx.sound_number_input_id())
                .unwrap()
                .get_revision(),
        };
        frame
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    presentation
                        .lexical_layout
                        .show(ui, graph_state, ctx, focus, outer_context);
                    ui.label(format!("Revision {}", rev.value()));
                    match outer_context {
                        OuterNumberGraphUiContext::SoundNumberInput(ctx) => {
                            let compiled_fn = ctx
                                .jit_client()
                                .get_compiled_number_input(ctx.sound_number_input_id(), rev);
                            match compiled_fn {
                                Some(f) => {
                                    let height = 20.0;
                                    let width = ui.available_width();
                                    let len = width.floor() as usize;
                                    let number_context = MockNumberContext::new(len);
                                    let mut dst = Vec::new();
                                    dst.resize(len, 0.0);
                                    f.eval(&mut dst, &number_context);
                                    let dx = width / (len - 1) as f32;
                                    let (_, rect) = ui.allocate_space(egui::vec2(width, height));
                                    let painter = ui.painter();
                                    painter.rect_filled(
                                        rect,
                                        egui::Rounding::none(),
                                        egui::Color32::BLACK,
                                    );
                                    for (i, (v0, v1)) in dst.iter().zip(&dst[1..]).enumerate() {
                                        let x0 = rect.left() + i as f32 * dx;
                                        let x1 = rect.left() + (i + 1) as f32 * dx;
                                        let y0 =
                                            rect.top() + height * (0.5 - v0.clamp(-1.0, 1.0) * 0.5);
                                        let y1 =
                                            rect.top() + height * (0.5 - v1.clamp(-1.0, 1.0) * 0.5);
                                        painter.line_segment(
                                            [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                                            egui::Stroke::new(2.0, egui::Color32::WHITE),
                                        );
                                    }
                                    painter.rect_stroke(
                                        rect,
                                        egui::Rounding::none(),
                                        egui::Stroke::new(2.0, egui::Color32::GRAY),
                                    );
                                }
                                None => {
                                    ui.label(".. no jit function yet ..");
                                }
                            }
                        }
                    }
                    // TODO: get compiled number input from server cache, plot it as a curve
                    // Also consider moving this code to LexicalLayout
                });
            })
            .inner;

        // TODO: consider traversing the lexical layout in search of graph inputs
        Vec::new()
    }
}
