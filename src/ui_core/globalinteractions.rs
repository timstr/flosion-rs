use std::collections::HashSet;

use eframe::egui;

use crate::core::{
    objecttype::ObjectType,
    sound::{
        soundgraph::SoundGraph, soundgraphid::SoundObjectId, soundprocessor::SoundProcessorId,
    },
};

use super::{
    expressiongraphuistate::ExpressionUiCollection,
    flosion_ui::Factories,
    graph_properties::GraphProperties,
    interactions::{
        draganddrop::{DragDropSubject, DragInteraction, DropInteraction},
        keyboardnav::KeyboardNavInteraction,
    },
    soundgraphuinames::SoundGraphUiNames,
    soundobjectpositions::SoundObjectPositions,
    soundobjectui::SoundObjectUiFactory,
    soundobjectuistate::SoundObjectUiStates,
    stackedlayout::stackedlayout::StackedLayout,
    summon_widget::{SummonWidget, SummonWidgetState, SummonWidgetStateBuilder},
};

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
    UsingKeyboardNav(KeyboardNavInteraction),

    /// Optionally clicking and dragging a rectangular area to define a new
    /// selection while a set of objects is selected and highlighted
    // TODO: selections should be integrated into drag & drop, cut & paste,
    // etc, and should thus be persistent across modes (and thus not a mode)
    Selecting(SelectingState),

    /// Something was clicked and is being dragged
    Dragging(DragInteraction),

    /// Something that was being dragged is being dropped
    Dropping(DropInteraction),

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
        properties: &GraphProperties,
        layout: &mut StackedLayout,
        object_states: &mut SoundObjectUiStates,
        positions: &mut SoundObjectPositions,
        expression_uis: &mut ExpressionUiCollection,
        names: &SoundGraphUiNames,
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
            UiMode::UsingKeyboardNav(keyboard_nav) => {
                keyboard_nav.interact_and_draw(
                    ui,
                    graph,
                    layout,
                    positions,
                    expression_uis,
                    factories,
                    names,
                    properties,
                );
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
                    graph
                        .try_make_change(|graph| {
                            let objects: Vec<SoundObjectId> =
                                selection.objects.iter().cloned().collect();
                            for oid in objects {
                                match oid {
                                    SoundObjectId::Sound(spid) => {
                                        graph.remove_sound_processor(spid)?
                                    }
                                }
                            }
                            Ok(())
                        })
                        .expect("Nah you can't delete those, sorry");
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

                // Highlight all selected objects
                for oid in &selection.objects {
                    let rect = match oid {
                        SoundObjectId::Sound(spid) => positions.find_processor(*spid).unwrap().rect,
                    };

                    ui.painter().rect_filled(
                        rect,
                        egui::Rounding::same(3.0),
                        egui::Color32::from_rgba_unmultiplied(255, 255, 0, 16),
                    );
                    ui.painter().rect_stroke(
                        rect,
                        egui::Rounding::same(3.0),
                        egui::Stroke::new(2.0, egui::Color32::YELLOW),
                    );
                }

                // Leave selection mode if nothing is selected or being selected
                if selection.objects.is_empty() && selection.selecting_area.is_none() {
                    self.mode = UiMode::Passive;
                }

                // TODO: cut, copy
            }
            UiMode::Dragging(drag) => {
                drag.interact_and_draw(ui, graph, object_states, layout, positions);
            }
            UiMode::Dropping(dropped_proc) => {
                dropped_proc.handle_drop(graph, layout, positions);
                self.mode = UiMode::Passive;
            }
            UiMode::Summoning(summon_widget) => {
                ui.add(SummonWidget::new(summon_widget));

                if let Some((object_type, args)) = summon_widget.final_choice() {
                    let new_obj = factories.sound_objects().create(object_type.name(), &args);

                    let object_ui = factories.sound_uis().get(new_obj.get_dynamic_type());
                    let state = object_ui.make_ui_state(&*new_obj, &args).unwrap();

                    object_states.set_object_data(new_obj.id(), state);

                    // Move the processor to the cursor location
                    let pos = summon_widget.position();
                    match new_obj.id() {
                        SoundObjectId::Sound(id) => positions.record_processor(
                            id,
                            egui::Rect::from_min_size(pos, egui::Vec2::ZERO),
                            pos,
                        ),
                    }

                    graph.add_sound_processor(new_obj.into_boxed_sound_processor().unwrap());

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
                objects: graph.graph_object_ids().collect(),
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

    pub(crate) fn start_dragging(&mut self, subject: DragDropSubject, original_rect: egui::Rect) {
        self.mode = UiMode::Dragging(DragInteraction::new(subject, original_rect));
    }

    pub(crate) fn continue_drag_move_by(&mut self, delta: egui::Vec2) {
        let UiMode::Dragging(drag) = &mut self.mode else {
            panic!("Called continue_drag_move_by() while not dragging");
        };

        drag.set_rect(drag.rect().translate(delta));
    }

    pub(crate) fn continue_drag_move_to(&mut self, rect: egui::Rect) {
        let UiMode::Dragging(drag) = &mut self.mode else {
            panic!("Called continue_drag_move_to() while not dragging");
        };

        drag.set_rect(rect);
    }

    pub(crate) fn drop_dragging(&mut self) {
        let UiMode::Dragging(drag_data) = &mut self.mode else {
            panic!("Called drop_dragging() while not dragging");
        };
        self.mode = UiMode::Dropping(DropInteraction::new_from_drag(drag_data));
    }

    pub(crate) fn focus_on_processor(&mut self, processor: SoundProcessorId) {
        self.mode =
            UiMode::UsingKeyboardNav(KeyboardNavInteraction::AroundSoundProcessor(processor));
    }

    /// Remove any data associated with objects that are no longer present in
    /// the graph
    pub(crate) fn cleanup(&mut self, graph: &SoundGraph) {
        match &mut self.mode {
            UiMode::Passive => (),
            UiMode::Selecting(s) => {
                s.objects.retain(|id| graph.contains(id));
                if s.objects.is_empty() {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::UsingKeyboardNav(kbd_focus) => {
                if !kbd_focus.is_valid(graph) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Dragging(data) => {
                if !data.is_valid(graph) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Dropping(data) => {
                if !data.is_valid(graph) {
                    self.mode = UiMode::Passive;
                }
            }
            UiMode::Summoning(_) => (),
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
    fn start_summoning(&mut self, position: egui::Pos2, factory: &SoundObjectUiFactory) {
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
}
