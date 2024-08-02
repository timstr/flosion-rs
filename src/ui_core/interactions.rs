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

// TODO:
// - create a standalone function which carries out the changes of dropping
//   a given processor onto a given interconnect on the soundgraphtopology only
// - use that function to precompute which interconnects would be legal
//   by testing all of them and checking for errors.
// - pass the precomputed legal interconnects to the UI context/state, using
//   the new RevisedProperty construct to ensure it accurately reflects
//   the current graph and interconnects and does not suffer from weird
//   stale state bugs or wasted computation.
// - create a separate standalone function for carrying out a processor drop
//   onto an interconnect to the layout only
// - when drawing the layout while dragging a processor, highlight only those
//   interconnects which are legal
// - when a processor is droppped onto an interconnect, use the pair of functions
//   for editing the topology and layout to make the actual change. Using the
//   same topology-editing function for finding legal connections and for
//   actually making edits will guarantee consistency and prevent me from
//   duplicating code or writing another ridiculous and fragile graph traversal
//   algorithm.

fn drag_and_drop_processor_in_graph(
    topo: &mut SoundGraphTopology,
    processor: SoundProcessorId,
    interconnect: ProcessorInterconnect,
) -> Result<(), ()> {
    // Disconnect the processor from everything
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
            debug_assert_eq!(topo.sound_input(input.id).unwrap().target(), Some(top));
            topo.disconnect_sound_input(input.id).or(Err(()))?;

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
) {
    // TODO: assume that changes have already happened in graph,
    // e.g. drag_and_drop_processor_in_graph has already run
    // It would be cool if a simple call to regenerate() can do
    // most or all of the work here.
    todo!()
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
        positions: &mut SoundObjectPositions,
        interconnects: &[ProcessorInterconnect],
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
    }

    pub(crate) fn dragging_a_processor(&self) -> bool {
        match &self.mode {
            UiMode::DraggingProcessor(_) => true,
            _ => false,
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
        positions: &mut SoundObjectPositions,
    ) {
        let minimum_intersection = 1000.0; // idk

        let nearest_interconnect =
            positions.find_closest_interconnect(dropped_proc.rect, minimum_intersection);
        if let Some(nearest_interconnect) = nearest_interconnect {
            let interconnect = nearest_interconnect.interconnect;

            // No point in checking invariants later if they aren't
            // already upheld
            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph.topology()));

            if let Err(e) = graph.edit_topology(|topo| {
                drag_and_drop_processor_in_graph(topo, dropped_proc.processor_id, interconnect)
                    .unwrap();
                Ok(())
            }) {
                println!("Can't drop that there: {:?}", e);
                return;
            }
            drag_and_drop_processor_in_layout(layout, dropped_proc.processor_id, interconnect);

            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph.topology()));
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
