use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Write},
};

use crate::{
    core::{
        graphobject::{ObjectId, ObjectInitialization},
        graphserialization::{deserialize_sound_graph, serialize_sound_graph},
        object_factory::ObjectFactory,
        serialization::Archive,
        soundgraph::SoundGraph,
        soundgraphtopology::SoundGraphTopology,
    },
    objects::{dac::Dac, mixer::Mixer, whitenoise::WhiteNoise},
    ui_objects::all_objects::all_objects,
};
use eframe::{
    self,
    egui::{self, Response, Ui},
};
use rfd::FileDialog;

use super::{
    graph_ui_state::{GraphUIState, SelectionChange},
    object_ui::random_object_color,
    object_ui_states::ObjectUiStates,
    summon_widget::{SummonWidget, SummonWidgetState},
    ui_context::UiContext,
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
    known_object_ids: HashSet<ObjectId>,
}

impl FlosionApp {
    pub fn new(_cc: &eframe::CreationContext) -> FlosionApp {
        // TODO: learn about what CreationContext offers
        let mut graph = SoundGraph::new();

        // TEST while I figure out how to connect inputs in the ui
        {
            let dac = graph
                .add_static_sound_processor::<Dac>(ObjectInitialization::Default)
                .unwrap();
            let mixer = graph
                .add_dynamic_sound_processor::<Mixer>(ObjectInitialization::Default)
                .unwrap();
            let mixer2 = graph
                .add_dynamic_sound_processor::<Mixer>(ObjectInitialization::Default)
                .unwrap();
            let mixer3 = graph
                .add_dynamic_sound_processor::<Mixer>(ObjectInitialization::Default)
                .unwrap();
            let whitenoise = graph
                .add_dynamic_sound_processor::<WhiteNoise>(ObjectInitialization::Default)
                .unwrap();
            let whitenoise2 = graph
                .add_dynamic_sound_processor::<WhiteNoise>(ObjectInitialization::Default)
                .unwrap();
            graph
                .connect_sound_input(dac.input.id(), mixer.id())
                .unwrap();
            graph
                .connect_sound_input(mixer.get_input_ids()[0], whitenoise.id())
                .unwrap();
            graph
                .connect_sound_input(mixer.get_input_ids()[1], mixer2.id())
                .unwrap();
            graph
                .connect_sound_input(mixer2.get_input_ids()[1], mixer3.id())
                .unwrap();
            graph
                .connect_sound_input(mixer3.get_input_ids()[0], whitenoise2.id())
                .unwrap();
        }

        let (object_factory, ui_factory) = all_objects();
        let mut app = FlosionApp {
            graph,
            ui_state: GraphUIState::new(),
            object_states: ObjectUiStates::new(),
            object_factory,
            ui_factory,
            summon_state: None,
            selection_area: None,
            known_object_ids: HashSet::new(),
        };

        // Initialize all necessary ui state
        app.cleanup();

        #[cfg(debug_assertions)]
        {
            assert!(app.ui_state.check_invariants(app.graph.topology()));
            assert!(app.object_states.check_invariants(app.graph.topology()));
        }

        app
    }

    fn draw_all_objects(
        ui: &mut Ui,
        factory: &UiFactory,
        graph: &SoundGraph,
        ui_state: &mut GraphUIState,
        object_states: &mut ObjectUiStates,
    ) {
        // NOTE: ObjectUiStates doesn't technically need to be borrowed mutably
        // here, but it uses interior mutability with individual object states
        // and borrowing mutably here increases safety.

        for object in graph.topology().graph_objects() {
            if let Some(layout) = ui_state
                .temporal_layout()
                .find_top_level_layout(object.id())
            {
                // TODO: store this width somewhere
                let width = 300;
                let is_top_level = true;
                let ctx = UiContext::new(
                    factory,
                    &object_states,
                    graph.topology(),
                    is_top_level,
                    layout.time_axis,
                    width,
                    layout.nesting_depth,
                );
                factory.ui(&object, ui_state, ui, &ctx);
            }
        }
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
                let mods = ui.input(|i| i.modifiers);
                let change = if mods.alt {
                    SelectionChange::Subtract
                } else if mods.shift {
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

    fn handle_summon_widget(&mut self, ui: &mut Ui, bg_response: &Response, bg_id: egui::LayerId) {
        let pointer_pos = bg_response.hover_pos();
        let mut open_summon_widget = false;
        if let Some(p) = pointer_pos {
            if ui.input(|i| i.key_pressed(egui::Key::Tab)) {
                if ui.ctx().layer_id_at(p) == Some(bg_id) {
                    open_summon_widget = true;
                }
            }
        }
        if bg_response.double_clicked() {
            open_summon_widget = true;
        } else if bg_response.clicked() || bg_response.clicked_elsewhere() {
            self.summon_state = None;
        }

        if open_summon_widget && self.summon_state.is_none() {
            self.summon_state = Some(SummonWidgetState::new(
                pointer_pos.unwrap(),
                &self.ui_factory,
            ));
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

    fn delete_selection(ui_state: &mut GraphUIState) {
        let selection: Vec<ObjectId> = ui_state.selection().iter().cloned().collect();
        ui_state.make_change(move |g, s| {
            g.remove_objects_batch(&selection).unwrap_or_else(|e| {
                println!("Nope! Can't remove that:\n    {:?}", e);
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
                .object_positions()
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
            .object_positions_mut()
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
                    ui.output_mut(|o| o.copied_text = s);
                }
            }
            egui::Event::Cut => {
                if let Some(s) =
                    Self::serialize(&self.ui_state, &self.object_states, &self.graph, true)
                {
                    ui.output_mut(|o| o.copied_text = s);
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
                repeat: _,
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
            }
            _ => (),
        }
    }

    fn cleanup(&mut self) {
        let current_object_ids: HashSet<ObjectId> =
            self.graph.topology().graph_object_ids().collect();

        for object_id in &current_object_ids {
            if !self.known_object_ids.contains(object_id) {
                self.ui_state
                    .create_state_for(*object_id, self.graph.topology());
                self.object_states.create_state_for(
                    *object_id,
                    self.graph.topology(),
                    &self.ui_factory,
                );
            }
        }

        let remaining_graph_ids = self.graph.topology().all_ids();
        self.ui_state
            .cleanup(&remaining_graph_ids, self.graph.topology());
        self.object_states.cleanup(&remaining_graph_ids);

        self.known_object_ids = current_object_ids;
    }
}

impl eframe::App for FlosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            Self::draw_all_objects(
                ui,
                &self.ui_factory,
                &self.graph,
                &mut self.ui_state,
                &mut self.object_states,
            );

            let screen_rect = ui.input(|i| i.screen_rect());
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
            let layer_id = ui.layer_id();
            self.handle_summon_widget(ui, &bg_response, layer_id);

            Self::draw_selection_rect(ui, &self.selection_area);

            if self.summon_state.is_none() {
                let events = ctx.input_mut(|i| std::mem::take(&mut i.events));
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

            self.cleanup();

            #[cfg(debug_assertions)]
            {
                assert!(self.ui_state.check_invariants(self.graph.topology()));
                assert!(self.object_states.check_invariants(self.graph.topology()));
            }
        });
    }
}
