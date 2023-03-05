use std::{
    fs::File,
    io::{Read, Write},
};

use crate::{
    core::{
        graphobject::{GraphId, ObjectId},
        graphserialization::{deserialize_sound_graph, serialize_sound_graph},
        numbersource::NumberVisibility,
        object_factory::ObjectFactory,
        serialization::Archive,
        soundgraph::SoundGraph,
        soundgraphtopology::SoundGraphTopology,
    },
    ui_core::diagnostics::{Diagnostic, DiagnosticMessage, DiagnosticRelevance},
    ui_objects::all_objects::all_objects,
};
use eframe::{
    self,
    egui::{self, Response, Ui},
};
use rfd::FileDialog;

use super::{
    graph_ui_state::{GraphUIState, SelectionChange},
    object_ui::{random_object_color, PegDirection},
    object_ui_states::ObjectUiStates,
    summon_widget::{SummonWidget, SummonWidgetState},
    ui_factory::UiFactory,
};

struct SelectionState {
    start_location: egui::Pos2,
    end_location: egui::Pos2,
}

pub struct FlosionApp {
    graph: SoundGraph,
    object_factory: ObjectFactory,
    ui_factory: UiFactory,
    ui_state: GraphUIState,
    object_states: ObjectUiStates,
    summon_state: Option<SummonWidgetState>,
    selection_area: Option<SelectionState>,
}

impl FlosionApp {
    pub fn new(_cc: &eframe::CreationContext) -> FlosionApp {
        // TODO: learn about what CreationContext offers
        let graph = SoundGraph::new();
        let (object_factory, ui_factory) = all_objects();
        FlosionApp {
            graph,
            ui_state: GraphUIState::new(),
            object_states: ObjectUiStates::new(),
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
        object_states: &mut ObjectUiStates,
    ) {
        for object in graph.topology().graph_objects() {
            factory.ui(&object, ui_state, object_states, ui);
        }
    }

    fn handle_hotkey(
        key: egui::Key,
        modifiers: egui::Modifiers,
        ui_state: &mut GraphUIState,
        topo: &SoundGraphTopology,
    ) -> bool {
        if key == egui::Key::Escape {
            return ui_state.cancel_hotkey(topo);
        }
        if modifiers.any() {
            return false;
        }
        ui_state.activate_hotkey(key, topo)
    }

    fn handle_shortcuts_selection(
        key: egui::Key,
        modifiers: egui::Modifiers,
        ui_state: &mut GraphUIState,
        topo: &SoundGraphTopology,
    ) -> bool {
        if !modifiers.command_only() {
            return false;
        }
        match key {
            egui::Key::A => {
                ui_state.select_all(topo);
                true
            }
            egui::Key::D => {
                ui_state.select_none();
                true
            }
            _ => false,
        }
    }

    fn handle_shortcuts_save_open(
        key: egui::Key,
        modifiers: egui::Modifiers,
        ui_state: &mut GraphUIState,
        object_states: &mut ObjectUiStates,
        graph: &mut SoundGraph,
        object_factory: &ObjectFactory,
        ui_factory: &UiFactory,
    ) -> bool {
        if !modifiers.command_only() {
            return false;
        }
        match key {
            egui::Key::S => {
                let path = FileDialog::new()
                    .add_filter("Flosion project files", &["flo"])
                    .save_file();
                let path = match path {
                    Some(mut p) => {
                        if p.extension().is_none() {
                            p.set_extension("flo");
                        }
                        p
                    }
                    None => {
                        println!("No file was selected");
                        return true;
                    }
                };
                // TODO: no need to base64 encode
                let data = Self::serialize(ui_state, object_states, &graph, false).unwrap();
                let mut file = match File::create(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        println!("Failed to create file at {}: {}", path.to_str().unwrap(), e);
                        return true;
                    }
                };
                if let Err(e) = file.write(data.as_bytes()) {
                    println!(
                        "Error while writing to file at {}: {}",
                        path.to_str().unwrap(),
                        e
                    );
                }
                true
            }
            egui::Key::O => {
                // TODO: delete everything else? prompt user first?
                let path = FileDialog::new()
                    .add_filter("Flosion project files", &["flo"])
                    .pick_file();
                let path = match path {
                    Some(p) => p,
                    None => {
                        println!("No file was selected");
                        return true;
                    }
                };
                let mut file = match File::open(&path) {
                    Ok(f) => f,
                    Err(e) => {
                        println!("Failed to open file: {}", e);
                        return true;
                    }
                };
                let mut data = String::new();
                if let Err(e) = file.read_to_string(&mut data) {
                    println!("Failed to read file at {}: {}", path.to_str().unwrap(), e);
                    return true;
                }
                if let Err(_) = Self::deserialize(
                    ui_state,
                    object_states,
                    &data,
                    graph,
                    object_factory,
                    ui_factory,
                ) {
                    println!("Error while deserializing objects");
                }
                true
            }
            _ => false,
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

    fn handle_summon_widget(&mut self, ui: &mut Ui, bg_response: &Response) {
        let pointer_pos = bg_response.interact_pointer_pos();
        if bg_response.double_clicked() {
            self.summon_state = match self.summon_state {
                Some(_) => None,
                None => Some(SummonWidgetState::new(
                    pointer_pos.unwrap(),
                    &self.ui_factory,
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
                    let new_object =
                        self.object_factory
                            .create_from_args(type_name, &mut self.graph, &args);
                    let new_object = match new_object {
                        Ok(o) => o,
                        Err(_) => {
                            println!("Failed to create an object of type {}", type_name);
                            return;
                        }
                    };
                    let new_state = self.ui_factory.create_state_from_args(&new_object, &args);
                    self.object_states.set_object_data(
                        new_object.id(),
                        new_state,
                        random_object_color(),
                    );
                    self.ui_state.clear_selection();
                    self.ui_state.select_object(new_object.id());
                }
                self.summon_state = None;
            }
        }
    }

    fn handle_dropped_pegs(ui: &mut Ui, ui_state: &mut GraphUIState, topo: &SoundGraphTopology) {
        let (id_src, p) = match ui_state.take_peg_being_dropped() {
            Some(x) => x,
            None => return,
        };
        let id_dst = ui_state.layout_state().find_peg_near(p, ui);
        match id_src {
            GraphId::NumberInput(niid) => {
                if let Some(nsid) = topo.number_input(niid).unwrap().target() {
                    // Dragging from a number input that already was connected
                    ui_state.make_change(move |g, s| {
                        // Disconnect the input
                        g.disconnect_number_input(niid)
                            .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                    });
                    if let Some(GraphId::NumberInput(niid2)) = id_dst {
                        // If dropped onto a different number input, connect the number output to it
                        ui_state.make_change(move |g, s| {
                            g.connect_number_input(niid2, nsid)
                                .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                        });
                    }
                } else if let Some(GraphId::NumberSource(nsid)) = id_dst {
                    // Dragging from a disconnected number input to a number output
                    ui_state.make_change(move |g, s| {
                        g.connect_number_input(niid, nsid)
                            .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                    });
                }
            }
            GraphId::NumberSource(nsid) => {
                if let Some(GraphId::NumberInput(niid)) = id_dst {
                    // Dragging from a number output to a number input
                    ui_state.make_change(move |g, s| {
                        g.connect_number_input(niid, nsid)
                            .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                    });
                }
            }
            GraphId::SoundInput(siid) => {
                if let Some(spid) = topo.sound_input(siid).unwrap().target() {
                    // Dragging from a sound input that was already connected
                    ui_state.make_change(move |g, s| {
                        // Disconnect the input
                        g.disconnect_sound_input(siid)
                            .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                    });
                    if let Some(GraphId::SoundInput(siid2)) = id_dst {
                        // if dropped onto a different sound output, connect the sound input to it
                        ui_state.make_change(move |g, s| {
                            g.connect_sound_input(siid2, spid)
                                .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                        });
                    }
                } else if let Some(GraphId::SoundProcessor(spid)) = id_dst {
                    // Dragging from a disconnected sound input to a sound output
                    ui_state.make_change(move |g, s| {
                        g.connect_sound_input(siid, spid)
                            .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
                    });
                }
            }
            GraphId::SoundProcessor(spid) => {
                if let Some(GraphId::SoundInput(siid)) = id_dst {
                    // Dragging from a sound output to a sound input
                    ui_state.make_change(move |g, s| {
                        g.connect_sound_input(siid, spid)
                            .unwrap_or_else(|e| s.issue_interpreted_error(e.into()));
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

    fn paint_wire(
        painter: &egui::Painter,
        start: egui::Pos2,
        start_direction: egui::Vec2,
        end: egui::Pos2,
        end_direction: egui::Vec2,
        stroke: egui::Stroke,
    ) {
        let dist = (end - start).length();
        let p0 = start.to_vec2();
        let p1 = start.to_vec2() + 0.5 * dist * start_direction;
        let p2 = end.to_vec2() + 0.5 * dist * end_direction;
        let p3 = end.to_vec2();

        let subdivs = (dist / 8.0).max(1.0) as usize;
        let k_inv_subdivs = 1.0 / subdivs as f32;
        let mut p_prev = start;
        for i in 0..subdivs {
            let t = (i + 1) as f32 * k_inv_subdivs;
            let t2 = t * t;
            let t3 = t * t2;
            let omt = 1.0 - t;
            let omt2 = omt * omt;
            let omt3 = omt * omt2;

            let p = (omt3 * p0) + (3.0 * omt2 * t * p1) + (3.0 * omt * t2 * p2) + (t3 * p3);
            let p = p.to_pos2();

            painter.line_segment([p_prev, p], stroke);

            p_prev = p;
        }
    }

    fn peg_direction_to_vec(direction: PegDirection) -> egui::Vec2 {
        match direction {
            PegDirection::Left => egui::vec2(-1.0, 0.0),
            PegDirection::Top => egui::vec2(0.0, -1.0),
            PegDirection::Right => egui::vec2(1.0, 0.0),
        }
    }

    fn draw_wires(ui: &mut Ui, ui_state: &GraphUIState, topo: &SoundGraphTopology) {
        // TODO: consider choosing which layer to paint the wire on, rather
        // than always painting the wire on top. However, choosing the layer
        // won't always be correct (an object might be positioned on top of
        // the peg it's connected to) and requires access to egui things
        // (e.g. memory().areas) which aren't yet exposed.
        // On the other hand, is there any correct way to paint wires between
        // two connected objects that are directly on top of one another?

        let t = ui.input().time * 5.0;
        let pulse = (t - t.floor()) > 0.5;

        ui.with_layer_id(
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new("wires")),
            |ui| {
                let painter = ui.painter();
                let drag_peg = ui_state.peg_being_dragged();
                for (siid, si) in topo.sound_inputs() {
                    if let Some(spid) = si.target() {
                        let layout = ui_state.layout_state();
                        let si_state = layout.sound_inputs().get(siid).unwrap();
                        let sp_state = layout.sound_outputs().get(&spid).unwrap();
                        let faint = drag_peg == Some(GraphId::SoundInput(*siid));

                        let mut stroke = egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(
                                0,
                                255,
                                0,
                                if faint { 64 } else { 255 },
                            ),
                        );

                        if let Some(r) = ui_state.graph_item_has_warning((*siid).into()) {
                            if pulse {
                                stroke = match r {
                                    DiagnosticRelevance::Primary => {
                                        egui::Stroke::new(5.0, egui::Color32::RED)
                                    }
                                    DiagnosticRelevance::Secondary => {
                                        egui::Stroke::new(5.0, egui::Color32::RED)
                                    }
                                };
                            }
                            ui.ctx().request_repaint();
                        }

                        Self::paint_wire(
                            painter,
                            si_state.layout.center(),
                            Self::peg_direction_to_vec(si_state.direction),
                            sp_state.layout.center(),
                            Self::peg_direction_to_vec(sp_state.direction),
                            stroke,
                        );
                    }
                }
                for (niid, ni) in topo.number_inputs() {
                    if ni.visibility() == NumberVisibility::Private {
                        continue;
                    }
                    if let Some(nsid) = ni.target() {
                        let layout = ui_state.layout_state();
                        let ni_state = layout.number_inputs().get(niid).unwrap();
                        let ns_state = layout.number_outputs().get(&nsid).unwrap();
                        let faint = drag_peg == Some(GraphId::NumberInput(*niid));

                        let mut stroke = egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(
                                0,
                                0,
                                255,
                                if faint { 64 } else { 255 },
                            ),
                        );

                        if let Some(r) = ui_state.graph_item_has_warning((*niid).into()) {
                            if pulse {
                                stroke = match r {
                                    DiagnosticRelevance::Primary => {
                                        egui::Stroke::new(5.0, egui::Color32::RED)
                                    }
                                    DiagnosticRelevance::Secondary => {
                                        egui::Stroke::new(5.0, egui::Color32::RED)
                                    }
                                };
                            }
                            ui.ctx().request_repaint();
                        }

                        Self::paint_wire(
                            painter,
                            ni_state.layout.center(),
                            Self::peg_direction_to_vec(ni_state.direction),
                            ns_state.layout.center(),
                            Self::peg_direction_to_vec(ns_state.direction),
                            stroke,
                        );
                    }
                }
                if let Some(gid) = ui_state.peg_being_dragged() {
                    let cursor_pos = match ui.input().pointer.interact_pos() {
                        Some(p) => p,
                        None => return,
                    };
                    let cursor_direction = egui::vec2(0.0, 0.0);
                    let layout = ui_state.layout_state();
                    let peg_state;

                    let color;
                    match gid {
                        GraphId::NumberInput(niid) => {
                            if let Some(nsid) = topo.number_input(niid).unwrap().target() {
                                peg_state = layout.number_outputs().get(&nsid).unwrap();
                            } else {
                                peg_state = layout.number_inputs().get(&niid).unwrap();
                            }
                            color = egui::Color32::from_rgb(0, 0, 255);
                        }
                        GraphId::NumberSource(nsid) => {
                            peg_state = layout.number_outputs().get(&nsid).unwrap();
                            color = egui::Color32::from_rgb(0, 0, 255);
                        }
                        GraphId::SoundInput(siid) => {
                            if let Some(spid) = topo.sound_input(siid).unwrap().target() {
                                peg_state = layout.sound_outputs().get(&spid).unwrap();
                            } else {
                                peg_state = layout.sound_inputs().get(&siid).unwrap();
                            }
                            color = egui::Color32::from_rgb(0, 255, 0);
                        }
                        GraphId::SoundProcessor(spid) => {
                            peg_state = layout.sound_outputs().get(&spid).unwrap();
                            color = egui::Color32::from_rgb(0, 255, 0);
                        }
                    }
                    let stroke = egui::Stroke::new(2.0, color);
                    Self::paint_wire(
                        painter,
                        peg_state.layout.center(),
                        Self::peg_direction_to_vec(peg_state.direction),
                        cursor_pos,
                        cursor_direction,
                        stroke,
                    );
                }
            },
        );
    }

    fn delete_selection(ui_state: &mut GraphUIState) {
        let selection: Vec<ObjectId> = ui_state.selection().iter().cloned().collect();
        ui_state.make_change(move |g, s| {
            g.remove_objects_batch(&selection).unwrap_or_else(|e| {
                println!("Nope! Can't remove that:\n    {:?}", e);
                for id in selection {
                    s.issue_diagnostic(Diagnostic::new(DiagnosticMessage::GraphItemWarning((
                        id.into(),
                        DiagnosticRelevance::Primary,
                    ))));
                }
                s.issue_interpreted_error(e);
            });
        });
    }

    fn serialize(
        ui_state: &GraphUIState,
        object_states: &ObjectUiStates,
        graph: &SoundGraph,
        use_selection: bool,
    ) -> Option<String> {
        #[cfg(debug_assertions)]
        {
            assert!(ui_state.check_invariants(graph.topology()));
        }

        let selection = if use_selection {
            let s = ui_state.selection();
            if s.is_empty() {
                return None;
            }
            Some(s)
        } else {
            None
        };
        let archive = Archive::serialize_with(|mut serializer| {
            let idmap = serialize_sound_graph(graph, selection.as_ref(), &mut serializer);
            ui_state
                .layout_state()
                .serialize(&mut serializer, selection.as_ref(), &idmap);
            object_states.serialize(&mut serializer, selection.as_ref(), &idmap);
        });
        let bytes = archive.into_vec();
        let b64_str = base64::encode(&bytes);
        Some(b64_str)
    }

    fn deserialize(
        ui_state: &mut GraphUIState,
        object_states: &mut ObjectUiStates,
        data: &str,
        graph: &mut SoundGraph,
        object_factory: &ObjectFactory,
        ui_factory: &UiFactory,
    ) -> Result<Vec<ObjectId>, ()> {
        let bytes = base64::decode(data).map_err(|_| ())?;
        let archive = Archive::from_vec(bytes);
        let mut deserializer = archive.deserialize()?;
        let (objects, idmap) = deserialize_sound_graph(graph, &mut deserializer, object_factory)?;
        ui_state
            .layout_state_mut()
            .deserialize(&mut deserializer, &idmap)?;
        object_states.deserialize(&mut deserializer, &idmap, graph.topology(), ui_factory)?;
        Ok(objects)
    }

    fn handle_event(&mut self, event: &egui::Event, ui: &egui::Ui) {
        match event {
            egui::Event::Copy => {
                if let Some(s) =
                    Self::serialize(&self.ui_state, &self.object_states, &self.graph, true)
                {
                    ui.output().copied_text = s;
                }
            }
            egui::Event::Cut => {
                if let Some(s) =
                    Self::serialize(&self.ui_state, &self.object_states, &self.graph, true)
                {
                    ui.output().copied_text = s;
                }
                Self::delete_selection(&mut self.ui_state);
            }
            egui::Event::Paste(data) => {
                let res = Self::deserialize(
                    &mut self.ui_state,
                    &mut self.object_states,
                    data,
                    &mut self.graph,
                    &self.object_factory,
                    &self.ui_factory,
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
                    return;
                }
                if Self::handle_shortcuts_selection(
                    *key,
                    *modifiers,
                    &mut self.ui_state,
                    self.graph.topology(),
                ) {
                    return;
                }
                if Self::handle_shortcuts_save_open(
                    *key,
                    *modifiers,
                    &mut self.ui_state,
                    &mut self.object_states,
                    &mut self.graph,
                    &self.object_factory,
                    &self.ui_factory,
                ) {
                    return;
                }
                if *key == egui::Key::Delete && !modifiers.any() {
                    Self::delete_selection(&mut self.ui_state);
                    return;
                }
                if Self::handle_hotkey(*key, *modifiers, &mut self.ui_state, self.graph.topology())
                {
                    return;
                }
            }
            egui::Event::PointerGone => self.ui_state.stop_dragging(None),
            _ => (),
        }
    }
}

impl eframe::App for FlosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_state.reset_pegs();
            {
                Self::draw_all_objects(
                    ui,
                    &self.ui_factory,
                    &self.graph,
                    &mut self.ui_state,
                    &mut self.object_states,
                );
            }
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

            {
                let topo = self.graph.topology();
                Self::handle_dropped_pegs(ui, &mut self.ui_state, &topo);
                Self::draw_selection_rect(ui, &self.selection_area);
                Self::draw_wires(ui, &self.ui_state, &topo);
            }

            if self.summon_state.is_none() {
                let events = std::mem::take(&mut ctx.input_mut().events);
                for event in &events {
                    self.handle_event(event, ui);
                }
            }

            #[cfg(debug_assertions)]
            {
                assert!(self.ui_state.check_invariants(self.graph.topology()));
                assert!(self.object_states.check_invariants(self.graph.topology()));
            }

            self.ui_state.apply_pending_changes(&mut self.graph);
            self.object_states
                .make_states_for_new_objects(self.graph.topology(), &self.ui_factory);
            let remaining_ids = self.graph.topology().all_ids();
            self.ui_state.cleanup(&remaining_ids);
            self.object_states.cleanup(&remaining_ids);

            #[cfg(debug_assertions)]
            {
                assert!(self.ui_state.check_invariants(self.graph.topology()));
            }
        });
    }
}
