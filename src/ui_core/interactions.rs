use std::collections::{HashMap, HashSet};

use eframe::egui;

use crate::core::{
    graph::graphobject::ObjectType,
    sound::{
        soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology,
        soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
};

use super::{
    keyboardfocus::KeyboardFocusState,
    soundgraphui::SoundGraphUi,
    summon_widget::{SummonWidgetState, SummonWidgetStateBuilder},
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
    pub from_input: Option<SoundInputId>,
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
    pub(crate) fn interact(&mut self, ui: &mut egui::Ui) {
        todo!()
    }

    /// Draw any additional visual interactive components to the screen, such
    /// as a selection rectangle or the summon widget
    pub(crate) fn draw(&self, ui: &mut egui::Ui) {
        todo!()
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
