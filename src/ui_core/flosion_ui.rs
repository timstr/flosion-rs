use crate::{
    core::{
        graphobject::{GraphId, ObjectId},
        soundgraph::SoundGraph,
    },
    ui_objects::object_factory::ObjectFactory,
};
use eframe::{egui, epi};
use futures::executor::block_on;

use super::{
    graph_ui_state::GraphUIState,
    summon_widget::{SummonWidget, SummonWidgetState},
};

struct SelectionState {
    start_location: egui::Pos2,
    end_location: egui::Pos2,
}

pub struct FlosionApp {
    graph: SoundGraph,
    all_object_uis: ObjectFactory,
    ui_state: GraphUIState,
    summon_state: Option<SummonWidgetState>,
    selection_area: Option<SelectionState>,
}

// async fn create_test_sound_graph() -> SoundGraph {
//     let mut sg: SoundGraph = SoundGraph::new();
//     let wavegen = sg.add_dynamic_sound_processor::<WaveGenerator>().await;
//     let dac = sg.add_static_sound_processor::<Dac>().await;
//     let dac_input_id = dac.instance().input().id();
//     let kb = sg.add_static_sound_processor::<Keyboard>().await;
//     let usine = sg.add_number_source::<UnitSine>().await;
//     sg.connect_sound_input(kb.instance().input.id(), wavegen.id())
//         .await
//         .unwrap();
//     sg.connect_sound_input(dac_input_id, kb.id()).await.unwrap();
//     sg.connect_number_input(wavegen.instance().amplitude.id(), usine.id())
//         .await
//         .unwrap();
//     sg.connect_number_input(usine.instance().input.id(), wavegen.instance().phase.id())
//         .await
//         .unwrap();
//     sg.connect_number_input(
//         wavegen.instance().frequency.id(),
//         kb.instance().key_frequency.id(),
//     )
//     .await
//     .unwrap();
//     println!(
//         "The Keyboard's keyed input has {} keys",
//         kb.instance().input.num_keys()
//     );
//     println!("The WaveGenerator has {} states", wavegen.num_states());
//     sg
// }

impl Default for FlosionApp {
    fn default() -> FlosionApp {
        // let graph = block_on(create_test_sound_graph());
        let graph = SoundGraph::new();
        FlosionApp {
            graph,
            all_object_uis: ObjectFactory::new(),
            ui_state: GraphUIState::new(),
            summon_state: None,
            selection_area: None,
        }
    }
}

impl epi::App for FlosionApp {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let running = self.graph.is_running();
            if ui.button(if running { "Pause" } else { "Play" }).clicked() {
                if running {
                    self.graph.stop();
                } else {
                    self.graph.start();
                }
            }
            self.ui_state.reset_pegs();
            for (object_id, object) in self.graph.graph_objects() {
                self.all_object_uis.ui(
                    *object_id,
                    object.as_ref(),
                    object.get_type(),
                    &mut self.ui_state,
                    ui,
                );
            }
            let bg_response = ui.interact(
                ui.input().screen_rect(),
                egui::Id::new("background"),
                egui::Sense::click_and_drag(),
            );
            let pointer_pos = bg_response.interact_pointer_pos();
            if bg_response.drag_started() {
                self.selection_area = Some(SelectionState {
                    start_location: pointer_pos.unwrap(),
                    end_location: pointer_pos.unwrap(),
                });
            }
            if bg_response.dragged() {
                if let Some(selection_area) = &mut self.selection_area {
                    selection_area.end_location = pointer_pos.unwrap();
                }
            }
            if bg_response.drag_released() {
                if let Some(selection_area) = self.selection_area.take() {
                    self.ui_state.select_with_rect(egui::Rect::from_two_pos(
                        selection_area.start_location,
                        selection_area.end_location,
                    ));
                }
            }
            if bg_response.double_clicked() {
                self.summon_state = match self.summon_state {
                    Some(_) => None,
                    None => Some(SummonWidgetState::new(
                        pointer_pos.unwrap(),
                        &self.all_object_uis,
                    )),
                };
            } else if bg_response.clicked() || bg_response.clicked_elsewhere() {
                self.summon_state = None;
            }
            if let Some(summon_state) = self.summon_state.as_mut() {
                ui.add(SummonWidget::new(summon_state));
            }
            if let Some(s) = &self.summon_state {
                if s.should_close() {
                    if s.selected_type().is_some() {
                        let (t, args) = s.parse_selected();
                        self.all_object_uis
                            .create(t, &args, &mut self.graph, &mut self.ui_state);
                    }
                    self.summon_state = None;
                }
            }

            let desc = self.graph.describe();
            if self.ui_state.peg_was_dropped() {
                let id_src = self.ui_state.dropped_peg_id().unwrap();
                let p = self.ui_state.drop_location().unwrap();
                let id_dst = self.ui_state.layout_state().find_peg_near(p, ui);
                match id_src {
                    GraphId::NumberInput(niid) => {
                        if desc.number_inputs().get(&niid).unwrap().target().is_some() {
                            block_on(self.graph.disconnect_number_input(niid))
                                .unwrap_or_else(|e| println!("Error: {:?}", e));
                        }
                        if let Some(GraphId::NumberSource(nsid)) = id_dst {
                            block_on(self.graph.connect_number_input(niid, nsid))
                                .unwrap_or_else(|e| println!("Error: {:?}", e));
                        }
                    }
                    GraphId::NumberSource(nsid) => {
                        if let Some(GraphId::NumberInput(niid)) = id_dst {
                            block_on(self.graph.connect_number_input(niid, nsid))
                                .unwrap_or_else(|e| println!("Error: {:?}", e));
                        }
                    }
                    GraphId::SoundInput(siid) => {
                        if desc.sound_inputs().get(&siid).unwrap().target().is_some() {
                            block_on(self.graph.disconnect_sound_input(siid)).unwrap();
                        }
                        if let Some(GraphId::SoundProcessor(spid)) = id_dst {
                            block_on(self.graph.connect_sound_input(siid, spid))
                                .unwrap_or_else(|e| println!("Error: {:?}", e));
                        }
                    }
                    GraphId::SoundProcessor(spid) => {
                        if let Some(GraphId::SoundInput(siid)) = id_dst {
                            block_on(self.graph.connect_sound_input(siid, spid))
                                .unwrap_or_else(|e| println!("Error: {:?}", e));
                        }
                    }
                }
            }

            if let Some(selection_area) = &self.selection_area {
                ui.with_layer_id(
                    egui::LayerId::new(egui::Order::Background, egui::Id::new("selection")),
                    |ui| {
                        let painter = ui.painter();
                        painter.rect(
                            egui::Rect::from_two_pos(
                                selection_area.start_location,
                                selection_area.end_location,
                            ),
                            0.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 16),
                            egui::Stroke::new(
                                2.0,
                                egui::Color32::from_rgba_unmultiplied(255, 255, 0, 64),
                            ),
                        )
                    },
                );
            }

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
                    for (siid, si) in desc.sound_inputs() {
                        if let Some(spid) = si.target() {
                            let layout = self.ui_state.layout_state();
                            let si_state = layout.sound_inputs().get(siid).unwrap();
                            let sp_state = layout.sound_outputs().get(&spid).unwrap();
                            let faint = drag_peg == Some(GraphId::SoundInput(*siid));
                            painter.line_segment(
                                [si_state.center(), sp_state.center()],
                                egui::Stroke::new(
                                    2.0,
                                    egui::Color32::from_rgba_unmultiplied(
                                        0,
                                        255,
                                        0,
                                        if faint { 64 } else { 255 },
                                    ),
                                ),
                            );
                        }
                    }
                    for (niid, ni) in desc.number_inputs() {
                        if let Some(nsid) = ni.target() {
                            let layout = self.ui_state.layout_state();
                            let ni_state = layout.number_inputs().get(niid).unwrap();
                            let ns_state = layout.number_outputs().get(&nsid).unwrap();
                            let faint = drag_peg == Some(GraphId::NumberInput(*niid));
                            painter.line_segment(
                                [ni_state.center(), ns_state.center()],
                                egui::Stroke::new(
                                    2.0,
                                    egui::Color32::from_rgba_unmultiplied(
                                        0,
                                        0,
                                        255,
                                        if faint { 64 } else { 255 },
                                    ),
                                ),
                            );
                        }
                    }
                    if let Some(gid) = self.ui_state.peg_being_dragged() {
                        let cursor_pos = ui.input().pointer.interact_pos().unwrap();
                        let layout = self.ui_state.layout_state();
                        let other_pos;
                        let color;
                        match gid {
                            GraphId::NumberInput(niid) => {
                                other_pos = layout.number_inputs().get(&niid).unwrap().center();
                                color = egui::Color32::from_rgb(0, 0, 255);
                            }
                            GraphId::NumberSource(nsid) => {
                                other_pos = layout.number_outputs().get(&nsid).unwrap().center();
                                color = egui::Color32::from_rgb(0, 0, 255);
                            }
                            GraphId::SoundInput(siid) => {
                                other_pos = layout.sound_inputs().get(&siid).unwrap().center();
                                color = egui::Color32::from_rgb(0, 255, 0);
                            }
                            GraphId::SoundProcessor(spid) => {
                                other_pos = layout.sound_outputs().get(&spid).unwrap().center();
                                color = egui::Color32::from_rgb(0, 255, 0);
                            }
                        }
                        painter
                            .line_segment([cursor_pos, other_pos], egui::Stroke::new(2.0, color));
                    }
                },
            );

            self.ui_state.apply_pending_changes(&mut self.graph);

            if self.summon_state.is_none() && ui.input().key_pressed(egui::Key::Delete) {
                let selection = self.ui_state.selection();
                for id in selection {
                    match id {
                        ObjectId::Sound(id) => {
                            block_on(self.graph.remove_sound_processor(*id));
                        }
                        ObjectId::Number(id) => {
                            block_on(self.graph.remove_number_source(*id));
                        }
                    }
                }
                self.ui_state.forget_selection();
            }
        });
    }

    fn name(&self) -> &str {
        "Flosion"
    }
}
