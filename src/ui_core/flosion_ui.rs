use std::sync::Arc;

use crate::{
    core::{
        graphobject::{GraphId, ObjectId, ObjectInitialization},
        graphserialization::{deserialize_sound_graph, serialize_sound_graph},
        object_factory::ObjectFactory,
        serialization::Archive,
        soundgraph::SoundGraph,
        soundgraphdescription::SoundGraphDescription,
        soundgraphtopology::SoundGraphTopology,
    },
    objects::{
        adsr::ADSR,
        dac::Dac,
        functions::{Constant, Divide, Exp, Fract, Multiply, Negate, Pow, SineWave},
        keyboard::Keyboard,
        wavegenerator::WaveGenerator,
    },
    ui_objects::all_objects::all_objects,
};
use eframe::{
    self,
    egui::{self, Response, Ui},
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
    let dac = sg
        .add_sound_processor::<Dac>(ObjectInitialization::Default)
        .unwrap();
    let keyboard = sg
        .add_sound_processor::<Keyboard>(ObjectInitialization::Default)
        .unwrap();
    let adsr = sg
        .add_sound_processor::<ADSR>(ObjectInitialization::Default)
        .unwrap();
    let wavegen = sg
        .add_sound_processor::<WaveGenerator>(ObjectInitialization::Default)
        .unwrap();
    sg.connect_sound_input(dac.input.id(), keyboard.id())
        .unwrap();
    sg.connect_sound_input(keyboard.input.id(), adsr.id())
        .unwrap();
    sg.connect_sound_input(adsr.input.id(), wavegen.id())
        .unwrap();
    let sinwave = sg
        .add_pure_number_source::<SineWave>(ObjectInitialization::Default)
        .unwrap();
    let const_rate = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let mul_time1 = sg
        .add_pure_number_source::<Multiply>(ObjectInitialization::Default)
        .unwrap();
    let mul_time2 = sg
        .add_pure_number_source::<Multiply>(ObjectInitialization::Default)
        .unwrap();
    let fract = sg
        .add_pure_number_source::<Fract>(ObjectInitialization::Default)
        .unwrap();
    let const_slope = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let mul_fract = sg
        .add_pure_number_source::<Multiply>(ObjectInitialization::Default)
        .unwrap();
    let neg = sg
        .add_pure_number_source::<Negate>(ObjectInitialization::Default)
        .unwrap();
    let exp = sg
        .add_pure_number_source::<Exp>(ObjectInitialization::Default)
        .unwrap();
    let mul_freq1 = sg
        .add_pure_number_source::<Multiply>(ObjectInitialization::Default)
        .unwrap();
    let mul_freq2 = sg
        .add_pure_number_source::<Multiply>(ObjectInitialization::Default)
        .unwrap();
    let const_peak_freq = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let const_base_freq = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let div_freq = sg
        .add_pure_number_source::<Divide>(ObjectInitialization::Default)
        .unwrap();
    let pow = sg
        .add_pure_number_source::<Pow>(ObjectInitialization::Default)
        .unwrap();
    let const_attack_time = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let const_decay_time = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let const_sustain_level = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let const_release_time = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    let const_exponent = sg
        .add_pure_number_source::<Constant>(ObjectInitialization::Default)
        .unwrap();
    sg.connect_number_input(wavegen.amplitude.id(), pow.id())
        .unwrap();
    sg.connect_number_input(sinwave.input.id(), wavegen.phase.id())
        .unwrap();
    sg.connect_number_input(pow.input_1.id(), sinwave.id())
        .unwrap();
    sg.connect_number_input(pow.input_2.id(), const_exponent.id())
        .unwrap();

    sg.connect_number_input(mul_time1.input_1.id(), wavegen.time.id())
        .unwrap();
    sg.connect_number_input(mul_time1.input_2.id(), div_freq.id())
        .unwrap();
    sg.connect_number_input(mul_time2.input_1.id(), const_rate.id())
        .unwrap();
    sg.connect_number_input(mul_time2.input_2.id(), mul_time1.id())
        .unwrap();
    sg.connect_number_input(fract.input.id(), mul_time2.id())
        .unwrap();
    sg.connect_number_input(mul_fract.input_1.id(), fract.id())
        .unwrap();
    sg.connect_number_input(mul_fract.input_2.id(), const_slope.id())
        .unwrap();
    sg.connect_number_input(neg.input.id(), mul_fract.id())
        .unwrap();
    sg.connect_number_input(exp.input.id(), neg.id()).unwrap();
    sg.connect_number_input(mul_freq1.input_1.id(), const_peak_freq.id())
        .unwrap();
    sg.connect_number_input(mul_freq1.input_2.id(), exp.id())
        .unwrap();
    sg.connect_number_input(div_freq.input_1.id(), keyboard.key_frequency.id())
        .unwrap();
    sg.connect_number_input(div_freq.input_2.id(), const_base_freq.id())
        .unwrap();
    sg.connect_number_input(mul_freq2.input_1.id(), mul_freq1.id())
        .unwrap();
    sg.connect_number_input(mul_freq2.input_2.id(), div_freq.id())
        .unwrap();
    sg.connect_number_input(wavegen.frequency.id(), mul_freq2.id())
        .unwrap();
    sg.connect_number_input(adsr.attack_time.id(), const_attack_time.id())
        .unwrap();
    sg.connect_number_input(adsr.decay_time.id(), const_decay_time.id())
        .unwrap();
    sg.connect_number_input(adsr.sustain_level.id(), const_sustain_level.id())
        .unwrap();
    sg.connect_number_input(adsr.release_time.id(), const_release_time.id())
        .unwrap();
    const_attack_time.set_value(0.01);
    const_decay_time.set_value(0.05);
    const_sustain_level.set_value(0.6);
    const_release_time.set_value(0.25);
    const_base_freq.set_value(250.0);
    const_peak_freq.set_value(500.0);
    const_rate.set_value(100.0);
    const_slope.set_value(7.5);
    const_exponent.set_value(8.0);
    sg.start();
    sg
}

impl FlosionApp {
    pub fn new(_cc: &eframe::CreationContext) -> FlosionApp {
        // TODO: learn about what CreationContext offers
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

    fn handle_hotkey(
        key: egui::Key,
        modifiers: egui::Modifiers,
        ui_state: &mut GraphUIState,
        desc: &SoundGraphDescription,
    ) -> bool {
        if key == egui::Key::Escape {
            return ui_state.cancel_hotkey(desc);
        }
        if modifiers.any() {
            return false;
        }
        ui_state.activate_hotkey(key, desc)
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
                    let (type_name, args) = s.parse_selected();
                    // TODO: how to distinguish args for ui from args for object, if ever needed?
                    // See also note in Constant::new
                    let new_object = self.object_factory.read().create_from_args(
                        type_name,
                        &mut self.graph.topology().write(),
                        &args,
                    );
                    let new_object = match new_object {
                        Ok(o) => o,
                        Err(_) => {
                            println!("Failed to create an object of type {}", type_name);
                            return;
                        }
                    };
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
        // TODO: curvy wires
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

    fn delete_selection(ui_state: &mut GraphUIState) {
        let selection = ui_state.selection();
        ui_state.make_change(move |g| g.remove_objects(selection.into_iter()));
    }

    fn serialize_selection(ui_state: &GraphUIState, topo: &SoundGraphTopology) -> Option<String> {
        let selection = ui_state.selection();
        if selection.is_empty() {
            return None;
        }
        let archive = Archive::serialize_with(|mut serializer| {
            let idmap = serialize_sound_graph(topo, Some(&selection), &mut serializer);
            ui_state.serialize_ui_states(&mut serializer, Some(&selection), &idmap);
        });
        let bytes = archive.into_vec();
        let b64_str = base64::encode(&bytes);
        Some(b64_str)
    }

    fn deserialize(
        ui_state: &mut GraphUIState,
        data: &str,
        topo: &mut SoundGraphTopology,
        object_factory: &ObjectFactory,
        ui_factory: &UiFactory,
    ) -> Result<Vec<ObjectId>, ()> {
        let bytes = base64::decode(data).map_err(|_| ())?;
        let archive = Archive::from_vec(bytes);
        let mut deserializer = archive.deserialize()?;
        let (objects, idmap) = deserialize_sound_graph(topo, &mut deserializer, object_factory)?;
        ui_state.deserialize_ui_states(&mut deserializer, &idmap, topo, ui_factory)?;
        Ok(objects)
    }
}

impl eframe::App for FlosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            let screen_rect = ui.input().screen_rect();
            let bg_response = ui.interact(
                screen_rect,
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

            if self.summon_state.is_none() {
                let mut copied_text: Option<String> = None;
                for event in &ctx.input().events {
                    match event {
                        egui::Event::Copy => {
                            if let Some(s) = Self::serialize_selection(
                                &self.ui_state,
                                &self.graph.topology().read(),
                            ) {
                                copied_text = Some(s);
                            }
                        }
                        egui::Event::Cut => {
                            if let Some(s) = Self::serialize_selection(
                                &self.ui_state,
                                &self.graph.topology().read(),
                            ) {
                                copied_text = Some(s);
                            }
                            Self::delete_selection(&mut self.ui_state);
                        }
                        egui::Event::Paste(data) => {
                            let res = Self::deserialize(
                                &mut self.ui_state,
                                data,
                                &mut self.graph.topology().write(),
                                &self.object_factory.read(),
                                &self.ui_factory.read(),
                            );
                            match res {
                                Ok(object_ids) => self
                                    .ui_state
                                    .set_selection(object_ids.into_iter().collect()),
                                Err(_) => println!("Failed to paste data"),
                            }
                        }
                        egui::Event::Key {
                            key,
                            pressed,
                            modifiers,
                        } => {
                            if !pressed {
                                continue;
                            }
                            if *key == egui::Key::Delete && !modifiers.any() {
                                Self::delete_selection(&mut self.ui_state);
                                continue;
                            }
                            if Self::handle_hotkey(*key, *modifiers, &mut self.ui_state, &desc) {
                                continue;
                            }
                        }
                        _ => (),
                    }
                }
                if let Some(s) = copied_text {
                    ui.output().copied_text = s;
                }
            }

            self.ui_state.apply_pending_changes(&mut self.graph);
            self.ui_state.cleanup();
        });
    }
}
