use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Write},
};

use crate::{
    core::{
        graph::{
            graphobject::{GraphObjectHandle, ObjectInitialization, ObjectType},
            objectfactory::ObjectFactory,
        },
        number::{numbergraph::NumberGraph, numbergraphdata::NumberTarget},
        revision::Revision,
        sound::{
            soundgraph::SoundGraph, soundgraphid::SoundObjectId,
            soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
        },
    },
    objects::{
        dac::Dac,
        ensemble::Ensemble,
        functions::{Add, Multiply, SawWave, Variable},
        wavegenerator::WaveGenerator,
    },
    ui_objects::all_objects::{all_number_graph_objects, all_sound_graph_objects},
};
use eframe::{
    self,
    egui::{self, Response, Ui},
};
use rfd::FileDialog;

use super::{
    numbergraphui::NumberGraphUi,
    soundgraphui::SoundGraphUi,
    soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::{DroppingProcessorData, SelectionChange, SoundGraphUiState},
    soundobjectuistate::SoundObjectUiStates,
    summon_widget::{SummonWidget, SummonWidgetState, SummonWidgetStateBuilder},
    ui_factory::UiFactory,
};

struct SelectionState {
    start_location: egui::Pos2,
    end_location: egui::Pos2,
}

pub struct FlosionApp {
    graph: SoundGraph,
    object_factory: ObjectFactory<SoundGraph>,
    number_object_factory: ObjectFactory<NumberGraph>,
    ui_factory: UiFactory<SoundGraphUi>,
    number_ui_factory: UiFactory<NumberGraphUi>,
    ui_state: SoundGraphUiState,
    object_states: SoundObjectUiStates,
    summon_state: Option<SummonWidgetState<ObjectType>>,
    selection_area: Option<SelectionState>,
    known_object_ids: HashSet<SoundObjectId>,
    previous_clean_revision: Option<u64>,
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
            let wavgen = graph
                .add_dynamic_sound_processor::<WaveGenerator>(ObjectInitialization::Default)
                .unwrap();
            let ensemble = graph
                .add_dynamic_sound_processor::<Ensemble>(ObjectInitialization::Default)
                .unwrap();
            graph
                .connect_sound_input(dac.input.id(), ensemble.id())
                .unwrap();
            graph
                .connect_sound_input(ensemble.input.id(), wavgen.id())
                .unwrap();

            // TODO: consider renaming all sound number sources / sound number inputs
            // in order to:
            // 1. remove overloaded terminology w.r.t. numbergraphs
            // 2. make it more clear how these number sources differ and
            //    how they merely provide access to numeric data for the
            //    number graph internals to do anything with
            graph
                .edit_number_input(wavgen.amplitude.id(), |numberinputdata| {
                    let (numbergraph, mapping) = numberinputdata.number_graph_and_mapping_mut();
                    let wavgen_phase_giid = mapping.add_target(wavgen.phase.id(), numbergraph);
                    let saw = numbergraph
                        .add_number_source::<SawWave>(ObjectInitialization::Default)
                        .unwrap();
                    // brutal
                    numbergraph
                        .connect_graph_output(
                            numbergraph.topology().graph_outputs()[0].id(),
                            NumberTarget::Source(saw.id()),
                        )
                        .unwrap();
                    let variable = numbergraph
                        .add_number_source::<Variable>(ObjectInitialization::Default)
                        .unwrap();
                    variable.set_value(1.0);

                    let add1 = numbergraph
                        .add_number_source::<Add>(ObjectInitialization::Default)
                        .unwrap();

                    let add2 = numbergraph
                        .add_number_source::<Add>(ObjectInitialization::Default)
                        .unwrap();

                    let multiply = numbergraph
                        .add_number_source::<Multiply>(ObjectInitialization::Default)
                        .unwrap();

                    numbergraph
                        .connect_number_input(saw.input.id(), NumberTarget::Source(multiply.id()))
                        .unwrap();

                    numbergraph
                        .connect_number_input(
                            multiply.input_1.id(),
                            NumberTarget::Source(add2.id()),
                        )
                        .unwrap();

                    numbergraph
                        .connect_number_input(add2.input_1.id(), NumberTarget::Source(add1.id()))
                        .unwrap();

                    numbergraph
                        .connect_number_input(add2.input_2.id(), NumberTarget::Source(add1.id()))
                        .unwrap();

                    numbergraph
                        .connect_number_input(
                            add1.input_1.id(),
                            NumberTarget::Source(variable.id()),
                        )
                        .unwrap();

                    numbergraph
                        .connect_number_input(
                            add1.input_2.id(),
                            NumberTarget::Source(variable.id()),
                        )
                        .unwrap();

                    numbergraph
                        .connect_number_input(
                            multiply.input_2.id(),
                            NumberTarget::GraphInput(wavgen_phase_giid),
                        )
                        .unwrap();
                })
                .unwrap();
            graph
                .edit_number_input(wavgen.frequency.id(), |numberinputdata| {
                    let (numbergraph, mapping) = numberinputdata.number_graph_and_mapping_mut();
                    let voice_freq_giid =
                        mapping.add_target(ensemble.voice_frequency.id(), numbergraph);
                    // brutal
                    numbergraph
                        .connect_graph_output(
                            numbergraph.topology().graph_outputs()[0].id(),
                            NumberTarget::GraphInput(voice_freq_giid),
                        )
                        .unwrap();
                })
                .unwrap();
            graph
                .edit_number_input(ensemble.frequency_in.id(), |numberinputdata| {
                    let numbergraph = numberinputdata.number_graph_mut();
                    let variable = numbergraph
                        .add_number_source::<Variable>(ObjectInitialization::Default)
                        .unwrap();
                    variable.set_value(60.0);
                    numbergraph
                        .connect_graph_output(
                            numbergraph.topology().graph_outputs()[0].id(),
                            NumberTarget::Source(variable.id()),
                        )
                        .unwrap();
                })
                .unwrap();

            // let dac = graph
            //     .add_static_sound_processor::<Dac>(ObjectInitialization::Default)
            //     .unwrap();
            // let mixer = graph
            //     .add_dynamic_sound_processor::<Mixer>(ObjectInitialization::Default)
            //     .unwrap();
            // let mixer2 = graph
            //     .add_dynamic_sound_processor::<Mixer>(ObjectInitialization::Default)
            //     .unwrap();
            // let mixer3 = graph
            //     .add_dynamic_sound_processor::<Mixer>(ObjectInitialization::Default)
            //     .unwrap();
            // let whitenoise = graph
            //     .add_dynamic_sound_processor::<WhiteNoise>(ObjectInitialization::Default)
            //     .unwrap();
            // let whitenoise2 = graph
            //     .add_dynamic_sound_processor::<WhiteNoise>(ObjectInitialization::Default)
            //     .unwrap();
            // let resampler = graph
            //     .add_dynamic_sound_processor::<Resampler>(ObjectInitialization::Default)
            //     .unwrap();
            // let wavgen = graph
            //     .add_dynamic_sound_processor::<WaveGenerator>(ObjectInitialization::Default)
            //     .unwrap();
            // graph
            //     .connect_sound_input(dac.input.id(), mixer.id())
            //     .unwrap();
            // graph
            //     .connect_sound_input(mixer.get_input_ids()[0], whitenoise.id())
            //     .unwrap();
            // graph
            //     .connect_sound_input(mixer.get_input_ids()[1], mixer2.id())
            //     .unwrap();
            // graph
            //     .connect_sound_input(mixer2.get_input_ids()[1], resampler.id())
            //     .unwrap();
            // graph
            //     .connect_sound_input(resampler.input.id(), mixer3.id())
            //     .unwrap();
            // graph
            //     .connect_sound_input(mixer3.get_input_ids()[0], whitenoise2.id())
            //     .unwrap();
            // graph
            //     .connect_sound_input(mixer3.get_input_ids()[1], wavgen.id())
            //     .unwrap();
        }

        let (object_factory, ui_factory) = all_sound_graph_objects();
        let (number_object_factory, number_ui_factory) = all_number_graph_objects();
        let mut app = FlosionApp {
            graph,
            ui_state: SoundGraphUiState::new(),
            object_states: SoundObjectUiStates::new(),
            object_factory,
            number_object_factory,
            ui_factory,
            number_ui_factory,
            summon_state: None,
            selection_area: None,
            known_object_ids: HashSet::new(),
            previous_clean_revision: None,
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

    fn draw_all_objects(&mut self, ui: &mut Ui) {
        let graph_objects: Vec<GraphObjectHandle<SoundGraph>> =
            self.graph.topology().graph_objects().collect();
        for object in graph_objects {
            if let Some(layout) = self
                .ui_state
                .temporal_layout()
                .find_top_level_layout(object.id())
            {
                let is_top_level = true;
                let mut ctx = SoundGraphUiContext::new(
                    &self.ui_factory,
                    &self.number_object_factory,
                    &self.number_ui_factory,
                    &self.object_states,
                    &mut self.graph,
                    is_top_level,
                    layout.time_axis,
                    layout.width_pixels as f32,
                    layout.nesting_depth,
                );
                self.ui_factory
                    .ui(&object, &mut self.ui_state, ui, &mut ctx);
            }
        }
        self.ui_state
            .apply_processor_drag(ui, self.graph.topology());
    }

    fn handle_shortcuts_selection(
        key: egui::Key,
        modifiers: egui::Modifiers,
        ui_state: &mut SoundGraphUiState,
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
        ui_state: &mut SoundGraphUiState,
        object_states: &mut SoundObjectUiStates,
        graph: &mut SoundGraph,
        object_factory: &ObjectFactory<SoundGraph>,
        ui_factory: &UiFactory<SoundGraphUi>,
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
                let Some(path) = path else {
                    println!("No file was selected");
                    return true;
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
        ui_state: &mut SoundGraphUiState,
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

    fn build_summon_widget(
        position: egui::Pos2,
        factory: &UiFactory<SoundGraphUi>,
    ) -> SummonWidgetState<ObjectType> {
        let mut builder = SummonWidgetStateBuilder::new(position);
        for object_type in factory.all_object_types() {
            builder.add_basic_name(object_type.name().to_string(), object_type);
        }
        builder.build()
    }

    fn handle_summon_widget(&mut self, ui: &mut Ui, bg_response: &Response, bg_id: egui::LayerId) {
        let pointer_pos = bg_response.hover_pos();
        let mut open_summon_widget = false;
        if let Some(p) = pointer_pos {
            if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
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
            self.summon_state = Some(Self::build_summon_widget(
                pointer_pos.unwrap(),
                &self.ui_factory,
            ));
        }

        if let Some(summon_state) = self.summon_state.as_mut() {
            ui.add(SummonWidget::new(summon_state));
        }
        if let Some(s) = &self.summon_state {
            if let Some(choice) = s.final_choice() {
                let (chosen_type, parsed_args) = choice;
                let new_object = self.object_factory.create_from_args(
                    chosen_type.name(),
                    &mut self.graph,
                    parsed_args,
                );
                let new_object = match new_object {
                    Ok(o) => o,
                    Err(_) => {
                        println!("Failed to create an object of type {}", chosen_type.name());
                        return;
                    }
                };
                let new_state = self.ui_factory.create_default_state(&new_object);
                let p = s.position();
                self.ui_state
                    .object_positions_mut()
                    .track_object_location(new_object.id(), egui::Rect::from_two_pos(p, p));
                self.object_states
                    .set_object_data(new_object.id(), new_state);
                self.ui_state.stop_selecting();
                self.ui_state.select_object(new_object.id());
                self.summon_state = None;
            }
        }
    }

    fn handle_keyboard_focus(&mut self, ui: &egui::Ui) {
        self.ui_state.handle_keyboard_focus(
            ui,
            &mut self.graph,
            &self.number_object_factory,
            &self.number_ui_factory,
            &mut self.object_states,
        );
    }

    fn handle_dropped_processor(&mut self, ui: &egui::Ui, data: DroppingProcessorData) {
        let shift_is_down = ui.input(|i| i.modifiers.shift);

        fn break_number_connections(graph: &mut SoundGraph, sound_input: SoundInputId) {
            let crossings: Vec<_> = graph
                .topology()
                .number_connection_crossings(sound_input)
                .collect();
            for (niid, nsid) in crossings {
                graph.disconnect_number_input(niid, nsid).unwrap();
            }
        }

        if let Some(siid) = data.target_input {
            // dropped onto a sound input

            if let Some(previous_siid) = data.from_input {
                // being dragged from another input
                if siid == previous_siid {
                    return;
                }

                if !shift_is_down {
                    break_number_connections(&mut self.graph, previous_siid);
                    self.graph.disconnect_sound_input(previous_siid).unwrap();
                }
            } else if !shift_is_down {
                // being dragged from top level, shift isn't held

                // ensure top level layout is removed if possible
                if self
                    .graph
                    .topology()
                    .sound_processor_targets(data.processor_id)
                    .count()
                    == 0
                {
                    self.ui_state
                        .temporal_layout_mut()
                        .remove_top_level_layout(data.processor_id.into());
                }
            }

            self.graph
                .connect_sound_input(siid, data.processor_id)
                .unwrap();
        } else {
            // not dropped suitably close to an input
            if let Some(previous_siid) = data.from_input {
                if !shift_is_down {
                    break_number_connections(&mut self.graph, previous_siid);
                    self.graph.disconnect_sound_input(previous_siid).unwrap();
                }
            }

            if !self
                .ui_state
                .temporal_layout()
                .is_top_level(data.processor_id.into())
            {
                // place the processor where it was dragged to if it was dragged
                // at top level
                self.ui_state
                    .object_positions_mut()
                    .track_object_location(data.processor_id.into(), data.rect);

                // give the processor a top level layout
                self.ui_state
                    .temporal_layout_mut()
                    .create_top_level_layout(data.processor_id.into());
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

    fn draw_nested_processor_dragging(ui: &mut Ui, rect: egui::Rect, color: egui::Color32) {
        let rounding = egui::Rounding::same(3.0);
        let [r, g, b, _a] = color.to_array();
        let a = 64;
        let color = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        ui.with_layer_id(
            egui::LayerId::new(egui::Order::Foreground, egui::Id::new("nested_drag")),
            |ui| {
                ui.painter().rect_filled(rect, rounding, color);

                ui.painter().rect_stroke(
                    rect,
                    rounding,
                    egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)),
                );
            },
        );
    }

    fn delete_selection(ui_state: &mut SoundGraphUiState) {
        let selection: Vec<SoundObjectId> =
            ui_state.effective_selection().iter().cloned().collect();
        if selection.is_empty() {
            return;
        }
        ui_state.make_change(move |g, _s| {
            g.remove_objects_batch(&selection).unwrap_or_else(|e| {
                println!("Nope! Can't remove that:\n    {:?}", e);
            });
        });
    }

    fn serialize(
        _ui_state: &SoundGraphUiState,
        _object_states: &SoundObjectUiStates,
        _graph: &SoundGraph,
        _use_selection: bool,
    ) -> Option<String> {
        // TODO
        println!("TODO: fix serialization");
        None
        // #[cfg(debug_assertions)]
        // {
        //     assert!(ui_state.check_invariants(graph.topology()));
        // }

        // let selection = if use_selection {
        //     let s = ui_state.selection();
        //     if s.is_empty() {
        //         return None;
        //     }
        //     Some(s)
        // } else {
        //     None
        // };
        // let archive = Archive::serialize_with(|mut serializer| {
        //     let idmap = serialize_sound_graph(graph, selection.as_ref(), &mut serializer);
        //     ui_state
        //         .object_positions()
        //         .serialize(&mut serializer, selection.as_ref(), &idmap);
        //     object_states.serialize(&mut serializer, selection.as_ref(), &idmap);
        // });
        // let bytes = archive.into_vec();
        // let b64_str = base64::encode(&bytes);
        // Some(b64_str)
    }

    fn deserialize(
        _ui_state: &mut SoundGraphUiState,
        _object_states: &mut SoundObjectUiStates,
        _data: &str,
        _graph: &mut SoundGraph,
        _object_factory: &ObjectFactory<SoundGraph>,
        _ui_factory: &UiFactory<SoundGraphUi>,
    ) -> Result<Vec<SoundObjectId>, ()> {
        // TODO
        println!("TODO: fix serialization");
        Err(())
        // let bytes = base64::decode(data).map_err(|_| ())?;
        // let archive = Archive::from_vec(bytes);
        // let mut deserializer = archive.deserialize()?;
        // let (objects, idmap) = deserialize_sound_graph(graph, &mut deserializer, object_factory)?;
        // ui_state
        //     .object_positions_mut()
        //     .deserialize(&mut deserializer, &idmap)?;
        // object_states.deserialize(&mut deserializer, &idmap, graph.topology(), ui_factory)?;
        // Ok(objects)
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
        // TODO: consider making revisions cached using some kind of clever wrapping
        // type that invalidates the revision number when mutably dereferenced and
        // caches it when calculated
        let current_revision = self.graph.topology().get_revision();

        if self.previous_clean_revision == Some(current_revision) {
            return;
        }

        let current_object_ids: HashSet<SoundObjectId> =
            self.graph.topology().graph_object_ids().collect();

        for object_id in &current_object_ids {
            if !self.known_object_ids.contains(object_id) {
                self.object_states.create_state_for(
                    *object_id,
                    self.graph.topology(),
                    &self.ui_factory,
                    &self.number_ui_factory,
                );
                self.ui_state.create_state_for(
                    *object_id,
                    self.graph.topology(),
                    &self.object_states,
                );
            }
        }

        let remaining_graph_ids = self.graph.topology().all_ids();
        self.ui_state.cleanup(
            &remaining_graph_ids,
            self.graph.topology(),
            &self.object_states,
        );
        self.object_states.cleanup(self.graph.topology());

        self.known_object_ids = current_object_ids;
        self.previous_clean_revision = Some(current_revision);
    }
}

impl eframe::App for FlosionApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_all_objects(ui);

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
            if let Some(drag_data) = self.ui_state.take_dropped_nested_processor() {
                self.handle_dropped_processor(ui, drag_data);
            }
            self.handle_keyboard_focus(ui);

            Self::draw_selection_rect(ui, &self.selection_area);

            if let Some(data) = self.ui_state.dragging_processor_data() {
                let color = self
                    .object_states
                    .get_object_color(data.processor_id.into());
                Self::draw_nested_processor_dragging(ui, data.rect, color);
            }

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

            self.graph.flush_updates();

            self.cleanup();

            #[cfg(debug_assertions)]
            {
                assert!(self.ui_state.check_invariants(self.graph.topology()));
                assert!(self.object_states.check_invariants(self.graph.topology()));
            }
        });
    }
}
