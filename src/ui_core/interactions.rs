use std::collections::HashSet;

use eframe::egui;
use symphonia::core::conv::IntoSample;

use crate::{
    core::{
        graph::graphobject::ObjectType,
        sound::{
            soundgraph::SoundGraph, soundgraphid::SoundObjectId,
            soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
            soundprocessor::SoundProcessorId,
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

pub struct DraggingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    original_rect: egui::Rect,
}

#[derive(Clone, Copy)]
pub struct DroppingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    pub original_rect: egui::Rect,
}

struct SelectionArea {
    start_location: egui::Pos2,
    end_location: egui::Pos2,
}

/// The set of mutually-exclusive top level behaviours that the app allows
enum UiMode {
    /// Not doing anything, just watching
    Passive,

    /// Jumping between sound processors and their components using the keyboard
    UsingKeyboardNav(KeyboardFocusState),

    /// Clicking and dragging a rectangular area to define a new selection
    MakingSelection(SelectionArea),

    /// A set of objects is selected and highlighted
    HoldingSelection(HashSet<SoundObjectId>),

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
        positions: &SoundObjectPositions,
    ) {
        match &mut self.mode {
            UiMode::Passive => {
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                    let position = ui
                        .ctx()
                        .pointer_latest_pos()
                        .unwrap_or(egui::pos2(50.0, 50.0));
                    self.start_summoning(position, factories.sound_uis())
                }
            }
            UiMode::UsingKeyboardNav(_) => todo!(),
            UiMode::MakingSelection(_) => todo!(),
            UiMode::HoldingSelection(_) => todo!(),
            UiMode::DraggingProcessor(drag) => {
                let color = object_states.get_object_color(drag.processor_id.into());
                let color =
                    egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 64);
                ui.painter()
                    .rect_filled(drag.rect, egui::Rounding::same(5.0), color);
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

                    self.mode = UiMode::Passive;
                } else if summon_widget.was_cancelled() {
                    self.mode = UiMode::Passive;
                }
            }
        }
    }

    pub(crate) fn dragging_a_processor(&self) -> bool {
        match &self.mode {
            UiMode::DraggingProcessor(_) => true,
            _ => false,
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

    /// Remove any data associated with objects that are no longer present in
    /// the topology
    pub(crate) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        match &mut self.mode {
            UiMode::MakingSelection(_) => (),
            UiMode::HoldingSelection(s) => {
                s.retain(|id| topo.contains((*id).into()));
                if s.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Passive => (),
            UiMode::UsingKeyboardNav(kbd_focus) => {
                if !topo.contains(kbd_focus.graph_id()) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::DraggingProcessor(data) => {
                if !topo.contains(data.processor_id.into()) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::DroppingProcessor(data) => {
                if !topo.contains(data.processor_id.into()) {
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
        positions: &SoundObjectPositions,
    ) {
        let minimum_intersection = 1000.0; // idk
        let intersection_area = dropped_proc
            .rect
            .intersect(dropped_proc.original_rect)
            .area();
        if intersection_area > minimum_intersection {
            // Didn't really move the processor, nothing to do
            return;
        }
        let nearest_interconnect =
            positions.find_closest_interconnect(dropped_proc.rect, minimum_intersection);
        if let Some(nearest_interconnect) = nearest_interconnect {
            let interconnect = nearest_interconnect.interconnect;
            if [
                interconnect.processor_above(),
                interconnect.processor_below(),
            ]
            .contains(&Some(dropped_proc.processor_id))
            {
                // Moved the processor onto one of its adjacent interconnects,
                // nothing to do
                return;
            }

            // Otherwise, the processor was dropped onto a different interconnect,
            // reconnect and move it there
            Self::disconnect_processor_in_graph(dropped_proc.processor_id, graph);

            debug_assert!(layout.check_invariants(graph.topology()));
            layout.split_processor_into_own_group(dropped_proc.processor_id);
            debug_assert!(layout.check_invariants(graph.topology()));

            match interconnect {
                ProcessorInterconnect::TopOfStack(ic_proc, ic_input) => {
                    graph
                        .connect_sound_input(ic_input.id, dropped_proc.processor_id)
                        .unwrap(); // fingers crossed
                    layout.insert_processor_above(dropped_proc.processor_id, ic_proc);
                    debug_assert!(layout.check_invariants(graph.topology()));
                }
                ProcessorInterconnect::BetweenTwoProcessors { bottom, top, input } => {
                    debug_assert_eq!(
                        graph.topology().sound_input(input.id).unwrap().target(),
                        Some(top)
                    );
                    graph.disconnect_sound_input(input.id).unwrap();
                    graph
                        .connect_sound_input(input.id, dropped_proc.processor_id)
                        .unwrap();
                    let inputs_on_dropped_proc = graph
                        .topology()
                        .sound_processor(dropped_proc.processor_id)
                        .unwrap()
                        .sound_inputs()
                        .clone();
                    layout.insert_processor_above(dropped_proc.processor_id, bottom);
                    if inputs_on_dropped_proc.len() == 1 {
                        graph
                            .connect_sound_input(inputs_on_dropped_proc[0], top)
                            .unwrap();
                    } else {
                        layout.split_group_above_processor(dropped_proc.processor_id);
                    }
                    debug_assert!(layout.check_invariants(graph.topology()));
                }
                ProcessorInterconnect::BottomOfStack(ic_proc) => {
                    let inputs_on_dropped_proc = graph
                        .topology()
                        .sound_processor(dropped_proc.processor_id)
                        .unwrap()
                        .sound_inputs()
                        .clone();
                    if inputs_on_dropped_proc.len() == 1 {
                        assert!(
                            graph.topology().sound_processor_targets(ic_proc).count() == 0,
                            "TODO: handle this"
                        );
                        graph
                            .connect_sound_input(inputs_on_dropped_proc[0], ic_proc)
                            .unwrap();
                        layout.insert_processor_below(dropped_proc.processor_id, ic_proc);
                    }
                    debug_assert!(layout.check_invariants(graph.topology()));
                }
            }
        } else {
            Self::disconnect_processor_in_graph(dropped_proc.processor_id, graph);

            layout.split_processor_into_own_group(dropped_proc.processor_id);

            debug_assert!(layout.check_invariants(graph.topology()));

            // TODO: move the new group to where the processor was dropped
        }
        // Uhhh what to do?
        // Much of the following logic should be cleanly delegated to SoundGraphLayout
        // in terms of small, self-contained operations.
        // If anything turns out to be visually and/or geometrically unintuitive, it
        // (and the visualization) should be rethought
        // ----
        // rough workflow:
        // - determine how the processor's drop location relates to positions of interconnects
        // - if the processor was dropped in free space, disconnect it, remove it from its ui group,
        //   and place it in its own new group
        // - if the processor was dropped on an interconnect:
        //    - if the processor is static and the interconnect is (transitively) non-sync, refuse and bail out
        //    - if the processor has no input and was dropped onto the bottom interconnect of a stacked group
        //       (e.g. the only implied thing to do is connect to the non-existent input), refuse and bail out
        //    - otherwise, disconnect the processor, remove it from its ui group, and:
        //       - if the processor has exactly one input:
        //          - insert the processor at that location in the stack
        //       - otherwise, if the processor has zero or more than one input:
        //          - break the interconnect's connection, split everything above the interconnect into a
        //            separate group, and connect the processor to the input below the interconnect
        // ----
        // common queries/operations used above:
        // - where are the interconnects located?
        // - which sound input and/or processor, does an interconnect refer to?
        // - removing a processor from its stacked group
        // - splitting a stacked group
        // - inserting a processer within a stacked group
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
