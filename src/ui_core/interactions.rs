use std::collections::HashSet;

use eframe::egui;

use crate::core::{
    graph::graphobject::ObjectType,
    sound::{
        soundgraph::SoundGraph, soundgraphid::SoundObjectId,
        soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    flosion_ui::Factories,
    keyboardfocus::KeyboardFocusState,
    soundgraphui::SoundGraphUi,
    soundgraphuistate::SoundGraphUiState,
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

pub struct DroppingProcessorData {
    pub processor_id: SoundProcessorId,
    pub rect: egui::Rect,
    pub target_input: Option<SoundInputId>,
    pub from_input: Option<SoundInputId>,
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

pub(crate) struct AppInteractions {
    /// The major mode through which the app is being interacted with,
    /// e.g. whether the user is drawing a selection, or doing nothing
    mode: UiMode,
}

/// Public methods
impl AppInteractions {
    /// Create a new AppInteractions instance
    pub(crate) fn new() -> AppInteractions {
        AppInteractions {
            mode: UiMode::Passive,
        }
    }

    /// Receive user input and handle and respond to all top-level interactions
    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        graph: &mut SoundGraph,
        object_states: &mut SoundObjectUiStates,
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
            UiMode::DroppingProcessor(_) => todo!(),
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
        let UiMode::DraggingProcessor(_) = &mut self.mode else {
            panic!("Called drog_dragging_processor() while not dragging");
        };
        // TODO: (arrange to) actually modify things
        // self.mode = UiMode::DroppingProcessor(...);
        self.mode = UiMode::Passive;
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

    // TODO:
    // - cut/copy/paste
    // - file save/open
}

/// Internal methods
impl AppInteractions {
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
}
