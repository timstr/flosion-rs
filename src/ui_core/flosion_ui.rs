use std::sync::Arc;

use crate::{
    core::{
        graphobject::{GraphId, ObjectId, ObjectInitialization},
        object_factory::ObjectFactory,
        soundgraph::SoundGraph,
        soundgraphdescription::SoundGraphDescription,
    },
    objects::{
        adsr::ADSR,
        dac::Dac,
        functions::{Constant, Divide, Exp, Fract, Multiply, Negate, Pow, USin},
        keyboard::Keyboard,
        wavegenerator::WaveGenerator,
    },
    ui_objects::all_objects::all_objects,
};
use eframe::{
    egui::{self, CtxRef, Response, Ui},
    epi,
};
use parking_lot::RwLock;

use super::{
    graph_ui_state::{GraphUIState, SelectionChange},
    summon_widget::{SummonWidget, SummonWidgetState},
    ui_factory::UiFactory,
};

struct SelectionState {
    start_location: egui::Pos2,
    end_location: egui::Pos2,
}

pub struct FlosionApp {
    graph: SoundGraph,
    object_factory: Arc<RwLock<ObjectFactory>>,
    ui_factory: Arc<RwLock<UiFactory>>,
    ui_state: GraphUIState,
    summon_state: Option<SummonWidgetState>,
    selection_area: Option<SelectionState>,
}

fn create_test_sound_graph() -> SoundGraph {
    let mut sg = SoundGraph::new();
    let dac = sg.add_sound_processor::<Dac>(ObjectInitialization::Default);
    let keyboard = sg.add_sound_processor::<Keyboard>(ObjectInitialization::Default);
    let adsr = sg.add_sound_processor::<ADSR>(ObjectInitialization::Default);
    let wavegen = sg.add_sound_processor::<WaveGenerator>(ObjectInitialization::Default);
    sg.connect_sound_input(dac.instance().input.id(), keyboard.id())
        .unwrap();
    sg.connect_sound_input(keyboard.instance().input.id(), adsr.id())
        .unwrap();
    sg.connect_sound_input(adsr.instance().input.id(), wavegen.id())
        .unwrap();
    let usin = sg.add_pure_number_source::<USin>(ObjectInitialization::Default);
    let const_rate = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let mul_time1 = sg.add_pure_number_source::<Multiply>(ObjectInitialization::Default);
    let mul_time2 = sg.add_pure_number_source::<Multiply>(ObjectInitialization::Default);
    let fract = sg.add_pure_number_source::<Fract>(ObjectInitialization::Default);
    let const_slope = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let mul_fract = sg.add_pure_number_source::<Multiply>(ObjectInitialization::Default);
    let neg = sg.add_pure_number_source::<Negate>(ObjectInitialization::Default);
    let exp = sg.add_pure_number_source::<Exp>(ObjectInitialization::Default);
    let mul_freq1 = sg.add_pure_number_source::<Multiply>(ObjectInitialization::Default);
    let mul_freq2 = sg.add_pure_number_source::<Multiply>(ObjectInitialization::Default);
    let const_peak_freq = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let const_base_freq = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let div_freq = sg.add_pure_number_source::<Divide>(ObjectInitialization::Default);
    let pow = sg.add_pure_number_source::<Pow>(ObjectInitialization::Default);
    let const_attack_time = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let const_decay_time = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let const_sustain_level = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let const_release_time = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    let const_exponent = sg.add_pure_number_source::<Constant>(ObjectInitialization::Default);
    sg.connect_number_input(wavegen.instance().amplitude.id(), pow.id())
        .unwrap();
    sg.connect_number_input(usin.instance().input.id(), wavegen.instance().phase.id())
        .unwrap();
    sg.connect_number_input(pow.instance().input_1.id(), usin.id())
        .unwrap();
    sg.connect_number_input(pow.instance().input_2.id(), const_exponent.id())
        .unwrap();

    sg.connect_number_input(
        mul_time1.instance().input_1.id(),
        wavegen.instance().time.id(),
    )
    .unwrap();
    sg.connect_number_input(mul_time1.instance().input_2.id(), div_freq.id())
        .unwrap();
    sg.connect_number_input(mul_time2.instance().input_1.id(), const_rate.id())
        .unwrap();
    sg.connect_number_input(mul_time2.instance().input_2.id(), mul_time1.id())
        .unwrap();
    sg.connect_number_input(fract.instance().input.id(), mul_time2.id())
        .unwrap();
    sg.connect_number_input(mul_fract.instance().input_1.id(), fract.id())
        .unwrap();
    sg.connect_number_input(mul_fract.instance().input_2.id(), const_slope.id())
        .unwrap();
    sg.connect_number_input(neg.instance().input.id(), mul_fract.id())
        .unwrap();
    sg.connect_number_input(exp.instance().input.id(), neg.id())
        .unwrap();
    sg.connect_number_input(mul_freq1.instance().input_1.id(), const_peak_freq.id())
        .unwrap();
    sg.connect_number_input(mul_freq1.instance().input_2.id(), exp.id())
        .unwrap();
    sg.connect_number_input(
        div_freq.instance().input_1.id(),
        keyboard.instance().key_frequency.id(),
    )
    .unwrap();
    sg.connect_number_input(div_freq.instance().input_2.id(), const_base_freq.id())
        .unwrap();
    sg.connect_number_input(mul_freq2.instance().input_1.id(), mul_freq1.id())
        .unwrap();
    sg.connect_number_input(mul_freq2.instance().input_2.id(), div_freq.id())
        .unwrap();
    sg.connect_number_input(wavegen.instance().frequency.id(), mul_freq2.id())
        .unwrap();
    sg.connect_number_input(adsr.instance().attack_time.id(), const_attack_time.id())
        .unwrap();
    sg.connect_number_input(adsr.instance().decay_time.id(), const_decay_time.id())
        .unwrap();
    sg.connect_number_input(adsr.instance().sustain_level.id(), const_sustain_level.id())
        .unwrap();
    sg.connect_number_input(adsr.instance().release_time.id(), const_release_time.id())
        .unwrap();
    const_attack_time.instance().set_value(0.01);
    const_decay_time.instance().set_value(0.05);
    const_sustain_level.instance().set_value(0.6);
    const_release_time.instance().set_value(0.25);
    const_base_freq.instance().set_value(250.0);
    const_peak_freq.instance().set_value(500.0);
    const_rate.instance().set_value(100.0);
    const_slope.instance().set_value(7.5);
    const_exponent.instance().set_value(8.0);
    sg.start();
    sg
}

impl Default for FlosionApp {
    fn default() -> FlosionApp {
        let graph = create_test_sound_graph();
        // let graph = SoundGraph::new();
        let topo = graph.topology();
        let (object_factory, ui_factory) = all_objects();
        let object_factory = Arc::new(RwLock::new(object_factory));
        let ui_factory = Arc::new(RwLock::new(ui_factory));
        FlosionApp {
            graph,
            ui_state: GraphUIState::new(topo, Arc::clone(&ui_factory)),
            object_factory,
            ui_factory,
            summon_state: None,
            selection_area: None,
        }
    }
}

impl FlosionApp {
    fn draw_all_objects(
        ui: &mut Ui,
        factory: &UiFactory,
        graph: &SoundGraph,
        ui_state: &mut GraphUIState,
    ) {
        for object in graph.graph_objects() {
            factory.ui(object.as_ref(), ui_state, ui);
        }
    }

    fn handle_hotkeys(ctx: &CtxRef, ui_state: &mut GraphUIState, desc: &SoundGraphDescription) {
        for e in &ctx.input().events {
            if let egui::Event::Key {
                key,
                pressed,
                modifiers,
            } = e
            {
                if !pressed {
                    continue;
                }
                if *key == egui::Key::Escape {
                    ui_state.cancel_hotkey(desc);
                }
                if modifiers.any() {
                    continue;
                }
                ui_state.activate_hotkey(*key, desc);
            }
        }
    }

    fn handle_drag_objects(
        ui: &mut Ui,
        bg_response: &Response,
        selection_area: &mut Option<SelectionState>,
        ui_state: &mut GraphUIState,
    ) {
        let pointer_pos = bg_response.interact_pointer_pos();
        if bg_response.drag_started() {
            *selection_area = Some(SelectionState {
                start_location: pointer_pos.unwrap(),
                end_location: pointer_pos.unwrap(),
            });
        }
        if bg_response.dragged() {
            if let Some(selection_area) = selection_area {
                selection_area.end_location = pointer_pos.unwrap();
            }
        }
        if bg_response.drag_released() {
            if let Some(selection_area) = selection_area.take() {
                let change = if ui.input().modifiers.alt {
                    SelectionChange::Subtract
                } else if ui.input().modifiers.shift {
                    SelectionChange::Add
                } else {
                    SelectionChange::Replace
                };
                ui_state.select_with_rect(
                    egui::Rect::from_two_pos(
                        selection_area.start_location,
                        selection_area.end_location,
                    ),
                    change,
                );
            }
        }
    }

    // TODO: does graph need to be mutated here?
    // can GraphUIState.make_change be used instead?
    fn handle_summon_widget(&mut self, ui: &mut Ui, bg_response: &Response) {
        let pointer_pos = bg_response.interact_pointer_pos();
        if bg_response.double_clicked() {
            self.summon_state = match self.summon_state {
                Some(_) => None,
                None => Some(SummonWidgetState::new(
                    pointer_pos.unwrap(),
                    &self.ui_factory.read(),
                )),
            };
        } else if bg_response.clicked() || bg_response.clicked_elsewhere() {
            self.summon_state = None;
        }
        if let Some(summon_state) = self.summon_state.as_mut() {
            ui.add(SummonWidget::new(summon_state));
        }
        if let Some(s) = &self.summon_state {
            if s.ready() {
                if s.selected_type().is_some() {
                    let (t, args) = s.parse_selected();
                    // TODO: how to distinguish args for ui from args for object, if ever needed?
                    let new_object =
                        self.object_factory
                            .read()
                            .create_from_args(t, &mut self.graph, &args);
                    let new_state = self
                        .ui_factory
                        .read()
                        .create_state_from_args(&*new_object, &args);
                    self.ui_state
                        .set_object_state(new_object.get_id(), new_state);
                    self.ui_state.clear_selection();
                    self.ui_state.select_object(new_object.get_id());
                }
                self.summon_state = None;
            }
        }
    }

    fn handle_dropped_pegs(ui: &mut Ui, ui_state: &mut GraphUIState, desc: &SoundGraphDescription) {
        let (id_src, p) = match ui_state.peg_being_dropped() {
            Some(x) => x,
            None => return,
        };
        let id_dst = ui_state.layout_state().find_peg_near(p, ui);
        match id_src {
            GraphId::NumberInput(niid) => {
                if desc.number_inputs().get(&niid).unwrap().target().is_some() {
                    ui_state.make_change(move |g| {
                        g.disconnect_number_input(niid)
                            .unwrap_or_else(|e| println!("Error: {:?}", e))
                    });
                }
                if let Some(GraphId::NumberSource(nsid)) = id_dst {
                    ui_state.make_change(move |g| {
                        g.connect_number_input(niid, nsid)
                            .unwrap_or_else(|e| println!("Error: {:?}", e))
                    });
                }
            }
            GraphId::NumberSource(nsid) => {
                if let Some(GraphId::NumberInput(niid)) = id_dst {
                    ui_state.make_change(move |g| {
                        g.connect_number_input(niid, nsid)
                            .unwrap_or_else(|e| println!("Error: {:?}", e))
                    });
                }
            }
            GraphId::SoundInput(siid) => {
                if desc.sound_inputs().get(&siid).unwrap().target().is_some() {
                    ui_state.make_change(move |g| g.disconnect_sound_input(siid).unwrap());
                }
                if let Some(GraphId::SoundProcessor(spid)) = id_dst {
                    ui_state.make_change(move |g| {
                        g.connect_sound_input(siid, spid)
                            .unwrap_or_else(|e| println!("Error: {:?}", e))
                    });
                }
            }
            GraphId::SoundProcessor(spid) => {
                if let Some(GraphId::SoundInput(siid)) = id_dst {
                    ui_state.make_change(move |g| {
                        g.connect_sound_input(siid, spid)
                            .unwrap_or_else(|e| println!("Error: {:?}", e))
                    });
                }
            }
        }
    }

    fn draw_selection_rect(ui: &mut Ui, selection_area: &Option<SelectionState>) {
        if let Some(selection_area) = selection_area {
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
    }

    fn draw_wires(ui: &mut Ui, ui_state: &GraphUIState, desc: &SoundGraphDescription) {
        // TODO: consider choosing which layer to paint the wire on, rather
        // than always painting the wire on top. However, choosing the layer
        // won't always be correct (an object might be positioned on top of
        // the peg it's connected to) and requires access to egui things
        // (e.g. memory().areas) which aren't yet exposed.
        // On the other hand, is there any correct way to paint wires between
        // two connected objects that are directly on top of one another?
        ui.with_layer_id(
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new("wires")),
            |ui| {
                let painter = ui.painter();
                let drag_peg = ui_state.peg_being_dragged();
                for (siid, si) in desc.sound_inputs() {
                    if let Some(spid) = si.target() {
                        let layout = ui_state.layout_state();
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
                        let layout = ui_state.layout_state();
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
                if let Some(gid) = ui_state.peg_being_dragged() {
                    let cursor_pos = ui.input().pointer.interact_pos().unwrap();
                    let layout = ui_state.layout_state();
                    let other_pos;
                    let color;
                    match gid {
                        GraphId::NumberInput(niid) => {
                            if let Some(nsid) = desc.number_inputs().get(&niid).unwrap().target() {
                                other_pos = layout.number_outputs().get(&nsid).unwrap().center();
                            } else {
                                other_pos = layout.number_inputs().get(&niid).unwrap().center();
                            }
                            color = egui::Color32::from_rgb(0, 0, 255);
                        }
                        GraphId::NumberSource(nsid) => {
                            other_pos = layout.number_outputs().get(&nsid).unwrap().center();
                            color = egui::Color32::from_rgb(0, 0, 255);
                        }
                        GraphId::SoundInput(siid) => {
                            if let Some(spid) = desc.sound_inputs().get(&siid).unwrap().target() {
                                other_pos = layout.sound_outputs().get(&spid).unwrap().center();
                            } else {
                                other_pos = layout.sound_inputs().get(&siid).unwrap().center();
                            }
                            color = egui::Color32::from_rgb(0, 255, 0);
                        }
                        GraphId::SoundProcessor(spid) => {
                            other_pos = layout.sound_outputs().get(&spid).unwrap().center();
                            color = egui::Color32::from_rgb(0, 255, 0);
                        }
                    }
                    painter.line_segment([cursor_pos, other_pos], egui::Stroke::new(2.0, color));
                }
            },
        );
    }

    fn handle_delete_objects(&mut self, ui: &mut Ui) {
        if self.summon_state.is_some() {
            return;
        }
        if ui.input().key_pressed(egui::Key::Delete) {
            let selection = self.ui_state.selection().clone();
            for id in selection {
                match id {
                    ObjectId::Sound(id) => {
                        self.ui_state
                            .make_change(move |g| g.remove_sound_processor(id));
                    }
                    ObjectId::Number(id) => {
                        self.ui_state
                            .make_change(move |g| g.remove_number_source(id));
                    }
                }
                self.ui_state.forget(id);
            }
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
            {
                let factory = self.ui_factory.read();
                Self::draw_all_objects(ui, &factory, &self.graph, &mut self.ui_state);
            }
            let desc = self.graph.describe();
            Self::handle_hotkeys(ctx, &mut self.ui_state, &desc);
            let bg_response = ui.interact(
                ui.input().screen_rect(),
                egui::Id::new("background"),
                egui::Sense::click_and_drag(),
            );
            Self::handle_drag_objects(
                ui,
                &bg_response,
                &mut self.selection_area,
                &mut self.ui_state,
            );
            self.handle_summon_widget(ui, &bg_response);
            Self::handle_dropped_pegs(ui, &mut self.ui_state, &desc);

            Self::draw_selection_rect(ui, &self.selection_area);

            Self::draw_wires(ui, &self.ui_state, &desc);

            self.handle_delete_objects(ui);

            self.ui_state.apply_pending_changes(&mut self.graph);
        });
    }

    fn name(&self) -> &str {
        "Flosion"
    }
}
