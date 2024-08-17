use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
};

use eframe::egui;
use hashrevise::{Revisable, RevisedProperty, RevisionHash, RevisionHasher};

use crate::core::{
    graph::graphobject::ObjectType,
    sound::{
        soundgraph::SoundGraph, soundgraphid::SoundObjectId,
        soundgraphtopology::SoundGraphTopology, soundgraphvalidation::find_sound_error,
        soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
};

use super::{
    flosion_ui::Factories,
    keyboardfocus::KeyboardFocusState,
    soundgraphui::SoundGraphUi,
    soundobjectpositions::SoundObjectPositions,
    soundobjectuistate::SoundObjectUiStates,
    stackedlayout::stackedlayout::SoundGraphLayout,
    summon_widget::{SummonWidget, SummonWidgetState, SummonWidgetStateBuilder},
    ui_factory::UiFactory,
};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum DragDropSubject {
    Processor(SoundProcessorId),
    Plug(SoundProcessorId),
    Socket(SoundInputId),
}

impl DragDropSubject {
    fn as_processor(&self) -> Option<SoundProcessorId> {
        match self {
            DragDropSubject::Processor(spid) => Some(*spid),
            DragDropSubject::Plug(spid) => Some(*spid),
            DragDropSubject::Socket(_) => None,
        }
    }

    fn as_input(&self) -> Option<SoundInputId> {
        match self {
            DragDropSubject::Processor(_) => None,
            DragDropSubject::Plug(_) => None,
            DragDropSubject::Socket(siid) => Some(*siid),
        }
    }
}

impl Revisable for DragDropSubject {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        match self {
            DragDropSubject::Processor(spid) => {
                hasher.write_u8(0);
                hasher.write_revisable(spid);
            }
            DragDropSubject::Plug(spid) => {
                hasher.write_u8(1);
                hasher.write_revisable(spid);
            }
            DragDropSubject::Socket(siid) => {
                hasher.write_u8(2);
                hasher.write_revisable(siid);
            }
        }
        hasher.into_revision()
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DragDropLegality {
    Legal,
    Illegal,
    Irrelevant,
}

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

fn drag_and_drop_in_graph(
    topo: &mut SoundGraphTopology,
    layout: &SoundGraphLayout,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
) -> DragDropLegality {
    // Disconnect things as needed
    match drag_from {
        DragDropSubject::Processor(spid) => {
            // If dragging a processor, disconnect it from everything
            // (for now)
            let mut inputs_to_disconnect = Vec::new();
            for i in topo.sound_processor(spid).unwrap().sound_inputs() {
                if topo.sound_input(*i).unwrap().target().is_some() {
                    inputs_to_disconnect.push(*i);
                }
            }

            for i in topo.sound_processor_targets(spid) {
                inputs_to_disconnect.push(i)
            }

            for i in inputs_to_disconnect {
                topo.disconnect_sound_input(i).unwrap();
            }
        }
        DragDropSubject::Plug(_) => {
            // If dragging a processor plug, don't disconnect it
        }
        DragDropSubject::Socket(siid) => {
            // If dragging an input socket, disconnect it if occupied
            if topo.sound_input(siid).unwrap().target().is_some() {
                topo.disconnect_sound_input(siid).unwrap();
            }
        }
    }

    if let (Some(proc), Some(input)) = (drag_from.as_processor(), drop_onto.as_input()) {
        // Dropping a processor onto an input. Connect the two.

        // Disconnect the input if it's occupied
        if topo.sound_input(input).unwrap().target().is_some() {
            // TODO: re-connect the processor below in the layout after?
            // Only if dragging processor and not its plug?
            topo.disconnect_sound_input(input).unwrap();
        }

        // Connect the input to the processor
        topo.connect_sound_input(input, proc).unwrap();

        DragDropLegality::Legal
    } else if let (DragDropSubject::Processor(proc), DragDropSubject::Plug(plug)) =
        (drag_from, drop_onto)
    {
        // Dragging a processor onto a processor plug. Only
        // okay if the plug is at the bottom of a stack and
        // the processor being dragged has exactly one input
        if !layout.is_bottom_of_group(plug) {
            // TODO: soft error, e.g. just ignore this
            return DragDropLegality::Irrelevant;
        }

        let inputs = topo.sound_processor(proc).unwrap().sound_inputs();
        if inputs.len() != 1 {
            return DragDropLegality::Illegal;
        }

        // The input should already have been disconnected.
        // Connect it to the plug.
        topo.connect_sound_input(inputs[0], plug).unwrap();

        DragDropLegality::Legal
    } else if let (Some(input), Some(proc)) = (drag_from.as_input(), drop_onto.as_processor()) {
        // Dragging an input socket onto a processor or its
        // plug. Connect the two.

        // Disconnect the input if it's occupied
        if topo.sound_input(input).unwrap().target().is_some() {
            topo.disconnect_sound_input(input).unwrap();
        }

        // Connect the input to the processor
        topo.connect_sound_input(input, proc).unwrap();

        DragDropLegality::Legal
    } else {
        // Dragging and dropping an unsupported combination of things
        DragDropLegality::Irrelevant
    }
}

fn drag_and_drop_in_layout(
    layout: &mut SoundGraphLayout,
    topo: &SoundGraphTopology,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
    positions: &SoundObjectPositions,
) {
    match (drag_from, drop_onto) {
        (DragDropSubject::Processor(proc), DragDropSubject::Socket(input)) => {
            // Dragging a processor onto an input. Move the processor
            // and insert it at the group under the input.
            layout.split_processor_into_own_group(proc, positions);
            let input_data = topo.sound_input(input).unwrap();
            let proc_below = input_data.owner();
            layout.split_group_above_processor(proc_below, positions);
            layout.insert_processor_above(proc, proc_below);
        }
        (DragDropSubject::Processor(proc), DragDropSubject::Plug(plug)) => {
            // Dragging a processor onto the bottom plug of a stacked group
            // (assuming that this is only being called after drag_and_drop_in_graph
            // has deemed it legal). Move the processor to the bottom of the group.
            layout.split_processor_into_own_group(proc, positions);
            layout.insert_processor_below(proc, plug);
        }
        _ => {
            // Otherwise, only plugs/sockets are being dragged, no layout changes
            // are needed that can't be resolved normally by SoundGraphLayout::regenerate
        }
    }
}

fn compute_legal_drop_sites(
    topo: &SoundGraphTopology,
    layout: &SoundGraphLayout,
    drag_subject: DragDropSubject,
    drop_sites: &[DragDropSubject],
) -> HashMap<DragDropSubject, DragDropLegality> {
    let mut site_statuses = HashMap::new();
    for drop_site in drop_sites {
        let mut topo_clone = topo.clone();
        let status = match drag_and_drop_in_graph(&mut topo_clone, layout, drag_subject, *drop_site)
        {
            DragDropLegality::Legal => {
                // drag_and_drop_in_graph only does superficial error
                // detection, here we additionally check whether the
                // resulting topology is valid.
                if find_sound_error(&topo_clone).is_some() {
                    DragDropLegality::Illegal
                } else {
                    DragDropLegality::Legal
                }
            }
            DragDropLegality::Illegal => DragDropLegality::Illegal,
            DragDropLegality::Irrelevant => DragDropLegality::Irrelevant,
        };
        site_statuses.insert(*drop_site, status);
    }
    site_statuses
}

pub struct DraggingData {
    subject: DragDropSubject,
    rect: egui::Rect,
    original_rect: egui::Rect,
    legal_drop_sites: RevisedProperty<HashMap<DragDropSubject, DragDropLegality>>,
}

#[derive(Clone)]
pub struct DroppingData {
    subject: DragDropSubject,
    rect: egui::Rect,
    original_rect: egui::Rect,
    legal_sites: HashMap<DragDropSubject, DragDropLegality>,
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

    /// Something was clicked and is being dragged
    Dragging(DraggingData),

    /// Something that was being dragged is being dropped
    Dropping(DroppingData),

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
            UiMode::Dragging(drag) => {
                // Draw a basic coloured rectangle tracking the cursor as a preview of
                // the subject being dragged, without drawing its ui twice
                let drag_subject_processor = match drag.subject {
                    DragDropSubject::Processor(spid) => spid,
                    DragDropSubject::Plug(spid) => spid,
                    DragDropSubject::Socket(siid) => {
                        graph.topology().sound_input(siid).unwrap().owner()
                    }
                };
                let color = object_states.get_object_color(drag_subject_processor.into());
                let color =
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 64);
                ui.painter()
                    .rect_filled(drag.rect, egui::Rounding::same(5.0), color);

                // Ensure that the legal connections are up to date, since these are used
                // to highlight legal/illegal interconnects to drop onto
                drag.legal_drop_sites.refresh4(
                    compute_legal_drop_sites,
                    graph.topology(),
                    layout,
                    drag.subject,
                    positions.drag_drop_subjects().values(),
                );
            }
            UiMode::Dropping(dropped_proc) => {
                let shift_held = ui.input(|i| i.modifiers.shift);
                Self::handle_processor_drop(
                    dropped_proc.clone(),
                    graph,
                    layout,
                    positions,
                    shift_held,
                );
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

        let (pressed_ctrl_a, pressed_esc) = ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::CTRL, egui::Key::A),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });

        // If ctrl+A was pressed, select everything
        if pressed_ctrl_a {
            self.mode = UiMode::Selecting(SelectingState {
                objects: graph.topology().graph_object_ids().collect(),
                selecting_area: None,
            })
        }

        // If escape was pressed, go into passive mode
        if pressed_esc {
            self.mode = UiMode::Passive;
        }

        // If the background was just clicked, go into passive mode
        if bg_response.clicked() {
            self.mode = UiMode::Passive;
        }
    }

    pub(crate) fn legal_sites_to_drop_onto(
        &self,
    ) -> Option<&HashMap<DragDropSubject, DragDropLegality>> {
        match &self.mode {
            UiMode::Dragging(drag) => drag.legal_drop_sites.get_cached(),
            _ => None,
        }
    }

    pub(crate) fn start_dragging(&mut self, subject: DragDropSubject, original_rect: egui::Rect) {
        self.mode = UiMode::Dragging(DraggingData {
            subject,
            rect: original_rect,
            original_rect,
            legal_drop_sites: RevisedProperty::new(),
        });
    }

    pub(crate) fn continue_dragging(&mut self, delta: egui::Vec2) {
        let UiMode::Dragging(drag) = &mut self.mode else {
            panic!("Called continue_dragging() while not dragging");
        };

        drag.rect = drag.rect.translate(delta);
    }

    pub(crate) fn drop_dragging(&mut self) {
        let UiMode::Dragging(drag_data) = &mut self.mode else {
            panic!("Called drop_dragging() while not dragging");
        };
        self.mode = UiMode::Dropping(DroppingData {
            subject: drag_data.subject,
            rect: drag_data.rect,
            original_rect: drag_data.original_rect,
            legal_sites: drag_data.legal_drop_sites.get_cached().cloned().unwrap(),
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
            UiMode::Dragging(data) => {
                let still_there = match &data.subject {
                    DragDropSubject::Processor(spid) => topo.contains(spid),
                    DragDropSubject::Plug(spid) => topo.contains(spid),
                    DragDropSubject::Socket(siid) => topo.contains(siid),
                };
                if !still_there {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Dropping(data) => {
                let still_there = match &data.subject {
                    DragDropSubject::Processor(spid) => topo.contains(spid),
                    DragDropSubject::Plug(spid) => topo.contains(spid),
                    DragDropSubject::Socket(siid) => topo.contains(siid),
                };
                if !still_there {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Summoning(_) => (),
        }
    }

    fn handle_processor_drop(
        drop_data: DroppingData,
        graph: &mut SoundGraph,
        layout: &mut SoundGraphLayout,
        positions: &mut SoundObjectPositions,
        shift_held: bool,
    ) {
        let minimum_overlap_area = 1000.0; // idk

        let nearest_drop_site = positions.drag_drop_subjects().find_closest_where(
            drop_data.rect,
            minimum_overlap_area,
            |site| drop_data.legal_sites.get(site).cloned() == Some(DragDropLegality::Legal),
        );

        if let Some(nearest_drop_site) = nearest_drop_site {
            // No point in checking invariants later if they aren't
            // already upheld
            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph.topology()));

            let drag_and_drop_result = graph.edit_topology(|topo| {
                Ok(drag_and_drop_in_graph(
                    topo,
                    layout,
                    drop_data.subject,
                    *nearest_drop_site,
                ))
            });

            match drag_and_drop_result {
                Ok(DragDropLegality::Legal) => { /* nice */ }
                Ok(DragDropLegality::Illegal) => {
                    println!("Nope, can't drop that there.");
                    return;
                }
                Ok(DragDropLegality::Irrelevant) => {
                    println!("How did you do that???");
                    return;
                }
                Err(e) => {
                    println!("Can't drop that there: {}", e.explain(graph.topology()));
                    return;
                }
            }

            drag_and_drop_in_layout(
                layout,
                graph.topology(),
                drop_data.subject,
                *nearest_drop_site,
                positions,
            );

            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph.topology()));
        } else {
            // If a processor was dropped far away from anything, split
            // it into its own group
            if let DragDropSubject::Processor(spid) = drop_data.subject {
                Self::disconnect_processor_in_graph(spid, graph);

                layout.split_processor_into_own_group(spid, positions);

                #[cfg(debug_assertions)]
                assert!(layout.check_invariants(graph.topology()));
            }
        }

        if !shift_held {
            // If a processor in a lone group was dropped, move the group to
            // where the processor was dropped
            if let DragDropSubject::Processor(spid) = drop_data.subject {
                let group = layout.find_group_mut(spid).unwrap();
                if group.processors() == &[spid] {
                    let rect = group.rect();
                    group.set_rect(
                        rect.translate(
                            drop_data.rect.left_top() - drop_data.original_rect.left_top(),
                        ),
                    );
                }
            }
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
