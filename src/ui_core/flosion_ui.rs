use crate::{
    core::{graphobject::GraphId, soundgraph::SoundGraph},
    objects::{
        dac::Dac,
        functions::{Constant, UnitSine},
        wavegenerator::WaveGenerator,
    },
    ui_objects::all_objects::AllObjectUis,
};
use eframe::{egui, epi};
use futures::executor::block_on;

use super::graph_ui_state::GraphUIState;

pub struct FlosionApp {
    graph: SoundGraph,
    all_object_uis: AllObjectUis,
    ui_state: GraphUIState,
}

async fn create_test_sound_graph() -> SoundGraph {
    let mut sg: SoundGraph = SoundGraph::new();
    let wavegen = sg.add_dynamic_sound_processor::<WaveGenerator>().await;
    let dac = sg.add_static_sound_processor::<Dac>().await;
    let dac_input_id = dac.instance().input().id();
    let constant = sg.add_number_source::<Constant>().await;
    let usine = sg.add_number_source::<UnitSine>().await;
    sg.connect_number_input(wavegen.instance().amplitude.id(), usine.id())
        .await
        .unwrap();
    sg.connect_number_input(usine.instance().input.id(), wavegen.instance().phase.id())
        .await
        .unwrap();
    sg.connect_number_input(wavegen.instance().frequency.id(), constant.id())
        .await
        .unwrap();
    constant.instance().set_value(440.0);
    sg.connect_sound_input(dac_input_id, wavegen.id())
        .await
        .unwrap();
    sg
}

impl Default for FlosionApp {
    fn default() -> FlosionApp {
        let graph = block_on(create_test_sound_graph());
        FlosionApp {
            graph,
            all_object_uis: AllObjectUis::new(),
            ui_state: GraphUIState::new(),
        }
    }
}

impl epi::App for FlosionApp {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hi earthguy");
            let running = self.graph.is_running();
            if ui.button(if running { "Pause" } else { "Play" }).clicked() {
                if running {
                    self.graph.stop();
                } else {
                    self.graph.start();
                }
            }
            self.ui_state.clear();
            for (object_id, object) in self.graph.graph_objects() {
                self.all_object_uis.ui(
                    *object_id,
                    object.as_ref(),
                    object.get_type(),
                    &mut self.ui_state,
                    ui,
                );
            }

            let desc = self.graph.describe();
            // TODO: consider choosing which layer to paint the wire on, rather
            // than always painting the wire on top. However, choosing the layer
            // won't always be correct (an object might be positioned on top of
            // the peg it's connected to) and requires access to egui things
            // (e.g. memory().areas) which aren't yet exposed.
            ui.with_layer_id(
                egui::LayerId::new(egui::Order::Foreground, egui::Id::new("wires")),
                |ui| {
                    let painter = ui.painter();
                    let drag_peg = self.ui_state.peg_being_dragged();
                    let cursor_pos = ui.input().pointer.interact_pos();
                    for (siid, si) in desc.sound_inputs() {
                        if let Some(spid) = si.target() {
                            let si_state = self.ui_state.sound_inputs().get(siid).unwrap();
                            let sp_state = self.ui_state.sound_outputs().get(&spid).unwrap();
                            let mut si_pos = si_state.center();
                            let mut sp_pos = sp_state.center();
                            if drag_peg == Some(GraphId::SoundInput(*siid)) {
                                if let Some(p) = cursor_pos {
                                    si_pos = p;
                                }
                            }
                            if drag_peg == Some(GraphId::SoundProcessor(spid)) {
                                if let Some(p) = cursor_pos {
                                    sp_pos = p;
                                }
                            }
                            painter.line_segment(
                                [si_pos, sp_pos],
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 255, 0)),
                            );
                        }
                    }
                    for (niid, ni) in desc.number_inputs() {
                        if let Some(nsid) = ni.target() {
                            let ni_state = self.ui_state.number_inputs().get(niid).unwrap();
                            let ns_state = self.ui_state.number_outputs().get(&nsid).unwrap();
                            let mut ni_pos = ni_state.center();
                            let mut ns_pos = ns_state.center();
                            if drag_peg == Some(GraphId::NumberInput(*niid)) {
                                if let Some(p) = cursor_pos {
                                    ni_pos = p;
                                }
                            }
                            if drag_peg == Some(GraphId::NumberSource(nsid)) {
                                if let Some(p) = cursor_pos {
                                    ns_pos = p;
                                }
                            }
                            painter.line_segment(
                                [ni_pos, ns_pos],
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(0, 0, 255)),
                            );
                        }
                    }
                },
            );
        });
    }

    fn name(&self) -> &str {
        "Flosion"
    }
}
