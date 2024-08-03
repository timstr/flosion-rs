use std::collections::HashSet;

use eframe::egui;
use hashrevise::RevisedProperty;

use crate::{
    core::{
        graph::graphobject::ObjectType,
        sound::{
            soundgraph::SoundGraph, soundgraphid::SoundObjectId,
            soundgraphtopology::SoundGraphTopology, soundgraphvalidation::find_sound_error,
            soundinput::SoundInputId, soundprocessor::SoundProcessorId,
        },
    },
    ui_core::soundgraphlayout::ProcessorInterconnect,
};

use super::{
    flosion_ui::Factories,
    keyboardfocus::KeyboardFocusState,
    soundgraphlayout::SoundGraphLayout,
    soundgraphui::SoundGraphUi,
    soundobjectpositions::SoundObjectPositions,
    soundobjectuistate::SoundObjectUiStates,
    summon_widget::{SummonWidget, SummonWidgetState, SummonWidgetStateBuilder},
    ui_factory::UiFactory,
};

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

fn drag_and_drop_processor_in_graph(
    topo: &mut SoundGraphTopology,
    processor: SoundProcessorId,
    interconnect: ProcessorInterconnect,
) -> Result<(), ()> {
    // Disconnect the processor from everything
    {
        let mut inputs_to_disconnect = Vec::new();
        for i in topo.sound_processor(processor).unwrap().sound_inputs() {
            if topo.sound_input(*i).unwrap().target().is_some() {
                inputs_to_disconnect.push(*i);
            }
        }

        for i in topo.sound_processor_targets(processor) {
            inputs_to_disconnect.push(i)
        }

        for i in inputs_to_disconnect {
            topo.disconnect_sound_input(i).or(Err(()))?;
        }
    }

    // Connect the processor at the interconnect
    match interconnect {
        ProcessorInterconnect::TopOfStack(_top_proc, input) => {
            topo.connect_sound_input(input.id, processor).or(Err(()))?;
        }
        ProcessorInterconnect::BetweenTwoProcessors {
            bottom: _,
            top,
            input,
        } => {
            // NOTE: this connection might just have been broken above!
            if topo.sound_input(input.id).unwrap().target().is_some() {
                topo.disconnect_sound_input(input.id).or(Err(()))?;
            }

            let dropped_inputs = topo.sound_processor(processor).unwrap().sound_inputs();
            if dropped_inputs.len() != 1 {
                // TODO: what should it mean to drag a processor with multiple inputs
                // onto the middle of a stack?
                return Err(());
            }
            let dropped_input = dropped_inputs[0];

            topo.connect_sound_input(input.id, processor).or(Err(()))?;
            topo.connect_sound_input(dropped_input, top).or(Err(()))?;
        }
        ProcessorInterconnect::BottomOfStack(bottom_proc) => {
            let inputs = topo.sound_processor(processor).unwrap().sound_inputs();
            if inputs.len() != 1 {
                // TODO: what should it mean to drag a processor with multiple inputs
                // onto the bottom end of a stack?
                return Err(());
            }
            let input = inputs[0];
            topo.connect_sound_input(input, bottom_proc).or(Err(()))?;
        }
    }

    Ok(())
}

fn drag_and_drop_processor_in_layout(
    layout: &mut SoundGraphLayout,
    processor: SoundProcessorId,
    interconnect: ProcessorInterconnect,
    positions: &SoundObjectPositions,
) {
    layout.split_processor_into_own_group(processor, positions);

    match interconnect {
        ProcessorInterconnect::TopOfStack(top_proc, _) => {
            layout.insert_processor_above(processor, top_proc);
        }
        ProcessorInterconnect::BetweenTwoProcessors {
            bottom,
            top: _,
            input: _,
        } => layout.insert_processor_above(processor, bottom),
        ProcessorInterconnect::BottomOfStack(bottom_proc) => {
            layout.insert_processor_below(processor, bottom_proc);
        }
    }
}

fn compute_legal_interconnects(
    topo: &SoundGraphTopology,
    processor: SoundProcessorId,
    interconnects: &[ProcessorInterconnect],
) -> Vec<ProcessorInterconnect> {
    let mut legal_interconnects = Vec::new();
    for interconnect in interconnects {
        let mut topo_clone = topo.clone();
        if drag_and_drop_processor_in_graph(&mut topo_clone, processor, *interconnect).is_err() {
            continue;
        }
        if find_sound_error(&topo_clone).is_none() {
            legal_interconnects.push(*interconnect);
        }
    }
    legal_interconnects
}

pub struct DraggingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    original_rect: egui::Rect,
    legal_connections: RevisedProperty<Vec<ProcessorInterconnect>>,
}

#[derive(Clone, Copy)]
pub struct DroppingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    pub original_rect: egui::Rect,
}

struct SelectingArea {
    start_location: egui::Pos2,
    end_location: egui::Pos2,
}

struct SelectingState {
    objects: HashSet<SoundObjectId>,
    selecting_area: Option<SelectingArea>,
}

/// The set of mutually-exclusive top level behaviours that the app allows
enum UiMode {
    /// Not doing anything, just watching
    Passive,

    /// Jumping between sound processors and their components using the keyboard
    UsingKeyboardNav(KeyboardFocusState),

    /// Optionally clicking and dragging a rectangular area to define a new
    /// selection while a set of objects is selected and highlighted
    Selecting(SelectingState),

    /// A processor was clicked and is being dragged
    DraggingProcessor(DraggingProcessorData),

    /// A processor that was being dragged is being dropped
    DroppingProcessor(DroppingProcessorData),

    /// The summon widget is open and an object's name is being typed
    /// along with any of its options
    Summoning(SummonWidgetState<ObjectType>),
}

pub(crate) struct GlobalInteractions {
    /// The major mode through which the app is being interacted with,
    /// e.g. whether the user is drawing a selection, or doing nothing
    mode: UiMode,
}

/// Public methods
impl GlobalInteractions {
    /// Create a new GlobalInteractions instance
    pub(crate) fn new() -> GlobalInteractions {
        GlobalInteractions {
            mode: UiMode::Passive,
        }
    }

    /// Receive user input and handle and respond to all top-level interactions
    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        graph: &mut SoundGraph,
        layout: &mut SoundGraphLayout,
        object_states: &mut SoundObjectUiStates,
        positions: &mut SoundObjectPositions,
        interconnects: &[ProcessorInterconnect],
        bg_response: egui::Response,
    ) {
        match &mut self.mode {
            UiMode::Passive => {
                let pressed_tab =
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab));

                if pressed_tab {
                    // If tab was pressed, start summon an object over the background
                    let position = ui
                        .ctx()
                        .pointer_latest_pos()
                        .unwrap_or(egui::pos2(50.0, 50.0));
                    self.start_summoning(position, factories.sound_uis())
                } else if bg_response.drag_started() {
                    // If the background was just clicked and dragged, start making a selection
                    let pointer_pos = bg_response.interact_pointer_pos().unwrap();
                    self.mode = UiMode::Selecting(SelectingState {
                        objects: HashSet::new(),
                        selecting_area: Some(SelectingArea {
                            start_location: pointer_pos,
                            end_location: pointer_pos + bg_response.drag_delta(),
                        }),
                    });
                }
            }
            UiMode::UsingKeyboardNav(_) => {
                // ????
                // TODO: handle arrow keys / enter / escape to change focus, tab to summon,
                // delete to delete, shortcuts for extracting/moving/reconnecting processors???
            }
            UiMode::Selecting(selection) => {
                let (pressed_esc, pressed_delete) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Delete),
                    )
                });

                if pressed_esc {
                    self.mode = UiMode::Passive;
                    return;
                } else if pressed_delete {
                    let objects: Vec<SoundObjectId> = selection.objects.iter().cloned().collect();
                    graph.remove_objects_batch(&objects).unwrap();
                    self.mode = UiMode::Passive;
                    return;
                } else {
                    // If the background was clicked and dragged, start another selection area while
                    // still holding the currently-selected objects
                    if bg_response.drag_started() {
                        let pos = bg_response.interact_pointer_pos().unwrap();
                        selection.selecting_area = Some(SelectingArea {
                            start_location: pos,
                            end_location: pos,
                        });
                    }

                    if let Some(area) = &mut selection.selecting_area {
                        Self::draw_selecting_area(ui, area);

                        area.end_location += bg_response.drag_delta();

                        let (shift_held, alt_held) =
                            ui.input(|i| (i.modifiers.shift, i.modifiers.alt));

                        if bg_response.drag_stopped() {
                            let new_objects =
                                Self::find_objects_touching_selection_area(area, positions);

                            if shift_held {
                                // If shift is held, add the new objects to the selection
                                selection.objects =
                                    selection.objects.union(&new_objects).cloned().collect();
                            } else if alt_held {
                                // If alt is held, remove the new objects from the selection
                                selection.objects = selection
                                    .objects
                                    .difference(&new_objects)
                                    .cloned()
                                    .collect();
                            } else {
                                // Otherwise, select only the new objects
                                selection.objects = new_objects;
                            }
                            selection.selecting_area = None;
                        }
                    }
                }

                if selection.objects.is_empty() && selection.selecting_area.is_none() {
                    self.mode = UiMode::Passive;
                }

                // TODO: cut, copy
            }
            UiMode::DraggingProcessor(drag) => {
                let color = object_states.get_object_color(drag.processor_id.into());
                let color =
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 64);
                ui.painter()
                    .rect_filled(drag.rect, egui::Rounding::same(5.0), color);

                drag.legal_connections.refresh3(
                    compute_legal_interconnects,
                    graph.topology(),
                    drag.processor_id,
                    interconnects,
                );
            }
            UiMode::DroppingProcessor(dropped_proc) => {
                Self::handle_processor_drop(*dropped_proc, graph, layout, positions);
                self.mode = UiMode::Passive;
            }
            UiMode::Summoning(summon_widget) => {
                ui.add(SummonWidget::new(summon_widget));

                if let Some((object_type, args)) = summon_widget.final_choice() {
                    let new_obj_handle = factories
                        .sound_objects()
                        .create_from_args(object_type.name(), graph, args)
                        .expect("Oops, failed to create object");

                    let state = factories.sound_uis().create_default_state(&new_obj_handle);

                    object_states.set_object_data(new_obj_handle.id(), state);

                    // Move the processor to the cursor location
                    let pos = summon_widget.position();
                    match new_obj_handle.id() {
                        SoundObjectId::Sound(id) => positions.record_processor(
                            id,
                            egui::Rect::from_min_size(pos, egui::Vec2::ZERO),
                            pos,
                        ),
                    }

                    self.mode = UiMode::Passive;
                } else if summon_widget.was_cancelled() {
                    self.mode = UiMode::Passive;
                }
            }
        }

        // If the background was just clicked, go into passive mode
        if bg_response.clicked() {
            self.mode = UiMode::Passive;
        }
    }

    pub(crate) fn processor_being_dragged(&self) -> Option<SoundProcessorId> {
        match &self.mode {
            UiMode::DraggingProcessor(drag) => Some(drag.processor_id),
            _ => None,
        }
    }

    pub(crate) fn legal_processors_to_drop_onto(&self) -> Option<&[ProcessorInterconnect]> {
        match &self.mode {
            UiMode::DraggingProcessor(drag) => drag
                .legal_connections
                .get_cached()
                .map(|v| -> &[ProcessorInterconnect] { &*v }),
            _ => None,
        }
    }

    pub(crate) fn start_dragging_processor(
        &mut self,
        processor_id: SoundProcessorId,
        original_rect: egui::Rect,
    ) {
        self.mode = UiMode::DraggingProcessor(DraggingProcessorData {
            processor_id,
            rect: original_rect,
            original_rect,
            legal_connections: RevisedProperty::new(),
        });
    }

    pub(crate) fn drag_processor(&mut self, delta: egui::Vec2) {
        let UiMode::DraggingProcessor(drag) = &mut self.mode else {
            panic!("Called drag_processor() while not dragging");
        };

        drag.rect = drag.rect.translate(delta);
    }

    pub(crate) fn drog_dragging_processor(&mut self) {
        let UiMode::DraggingProcessor(drag_data) = &mut self.mode else {
            panic!("Called drog_dragging_processor() while not dragging");
        };
        self.mode = UiMode::DroppingProcessor(DroppingProcessorData {
            processor_id: drag_data.processor_id,
            rect: drag_data.rect,
            original_rect: drag_data.original_rect,
        });
    }

    pub(crate) fn focus_on_processor(&mut self, processor: SoundProcessorId) {
        self.mode = UiMode::UsingKeyboardNav(KeyboardFocusState::AroundSoundProcessor(processor));
    }

    pub(crate) fn processor_is_in_focus(&self, processor: SoundProcessorId) -> bool {
        match &self.mode {
            UiMode::UsingKeyboardNav(KeyboardFocusState::AroundSoundProcessor(spid)) => {
                processor == *spid
            }
            _ => false,
        }
    }

    pub(crate) fn processor_is_selected(&self, processor: SoundProcessorId) -> bool {
        match &self.mode {
            UiMode::Selecting(selection) => selection.objects.contains(&processor.into()),
            _ => false,
        }
    }

    /// Remove any data associated with objects that are no longer present in
    /// the topology
    pub(crate) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        match &mut self.mode {
            UiMode::Selecting(s) => {
                s.objects.retain(|id| topo.contains(id));
                if s.objects.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Passive => (),
            UiMode::UsingKeyboardNav(kbd_focus) => {
                if !kbd_focus.is_valid(topo) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::DraggingProcessor(data) => {
                if !topo.contains(data.processor_id) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::DroppingProcessor(data) => {
                if !topo.contains(data.processor_id) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Summoning(_) => (),
        }
    }

    fn handle_processor_drop(
        dropped_proc: DroppingProcessorData,
        graph: &mut SoundGraph,
        layout: &mut SoundGraphLayout,
        positions: &mut SoundObjectPositions,
    ) {
        let minimum_intersection = 1000.0; // idk

        let nearest_interconnect =
            positions.find_closest_interconnect(dropped_proc.rect, minimum_intersection);
        if let Some(nearest_interconnect) = nearest_interconnect {
            let interconnect = nearest_interconnect.interconnect;

            if !interconnect.includes_processor(dropped_proc.processor_id) {
                // No point in checking invariants later if they aren't
                // already upheld
                #[cfg(debug_assertions)]
                assert!(layout.check_invariants(graph.topology()));

                let drag_and_drop_result = graph.edit_topology(|topo| {
                    Ok(drag_and_drop_processor_in_graph(
                        topo,
                        dropped_proc.processor_id,
                        interconnect,
                    ))
                });

                match drag_and_drop_result {
                    Ok(Ok(_)) => { /* nice */ }
                    Ok(Err(_)) => {
                        println!("Nope, can't drop that there.");
                        return;
                    }
                    Err(e) => {
                        println!("Can't drop that there: {:?}", e);
                        return;
                    }
                }

                drag_and_drop_processor_in_layout(
                    layout,
                    dropped_proc.processor_id,
                    interconnect,
                    positions,
                );

                #[cfg(debug_assertions)]
                assert!(layout.check_invariants(graph.topology()));
            }
        } else {
            Self::disconnect_processor_in_graph(dropped_proc.processor_id, graph);

            layout.split_processor_into_own_group(dropped_proc.processor_id, positions);

            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph.topology()));
        }

        // If the processor is in a lone group, move the group to where the processor
        // was dropped
        let group = layout.find_group_mut(dropped_proc.processor_id).unwrap();
        if group.processors() == &[dropped_proc.processor_id] {
            let rect = group.rect();
            group.set_rect(
                rect.translate(
                    dropped_proc.rect.left_top() - dropped_proc.original_rect.left_top(),
                ),
            );
        }
    }

    fn find_objects_touching_selection_area(
        area: &SelectingArea,
        positions: &SoundObjectPositions,
    ) -> HashSet<SoundObjectId> {
        let rect = egui::Rect::from_two_pos(area.start_location, area.end_location);
        positions
            .processors()
            .iter()
            .filter_map(|pp| -> Option<SoundObjectId> {
                if pp.rect.intersects(rect) {
                    Some(pp.processor.into())
                } else {
                    None
                }
            })
            .collect()
    }

    fn draw_selecting_area(ui: &mut egui::Ui, area: &SelectingArea) {
        let select_rect = egui::Rect::from_two_pos(area.start_location, area.end_location);

        ui.painter().rect_filled(
            select_rect,
            egui::Rounding::same(3.0),
            egui::Color32::from_rgba_unmultiplied(255, 255, 0, 16),
        );

        ui.painter().rect_stroke(
            select_rect,
            egui::Rounding::same(3.0),
            egui::Stroke::new(2.0, egui::Color32::YELLOW),
        );
    }

    // TODO:
    // - cut/copy/paste
    // - file save/open
}

/// Internal methods
impl GlobalInteractions {
    /// Switch to using the summon widget
    fn start_summoning(&mut self, position: egui::Pos2, factory: &UiFactory<SoundGraphUi>) {
        let mut builder = SummonWidgetStateBuilder::new(position);
        for object_ui in factory.all_object_uis() {
            for name in object_ui.summon_names() {
                builder.add_name_with_arguments(
                    name.to_string(),
                    object_ui.summon_arguments(),
                    object_ui.object_type(),
                );
            }
        }
        let widget = builder.build();
        self.mode = UiMode::Summoning(widget);
    }

    fn disconnect_processor_in_graph(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
        let mut inputs_to_disconnect_from: Vec<SoundInputId> = graph
            .topology()
            .sound_processor_targets(processor_id)
            .collect();
        for i in graph
            .topology()
            .sound_processor(processor_id)
            .unwrap()
            .sound_inputs()
        {
            if graph.topology().sound_input(*i).unwrap().target().is_some() {
                inputs_to_disconnect_from.push(*i);
            }
        }
        for i in inputs_to_disconnect_from {
            graph.disconnect_sound_input(i).unwrap();
        }
    }
}
