use eframe::egui;

use crate::core::{
    graph::{graphobject::ObjectType, objectfactory::ObjectFactory},
    number::{
        numbergraph::{NumberGraph, NumberGraphInputId},
        numbergraphtopology::NumberGraphTopology,
    },
    sound::{
        soundgraphdata::SoundNumberInputData, soundnumberinput::SoundNumberInputId,
        soundnumbersource::SoundNumberSourceId,
    },
};

use super::{
    lexicallayout::{LexicalLayout, LexicalLayoutCursor},
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{NumberGraphUiState, NumberObjectUiStates},
    summon_widget::SummonWidgetState,
    ui_factory::UiFactory,
};

#[derive(Copy, Clone)]
pub(super) enum NumberSummonValue {
    NumberSourceType(ObjectType),
    SoundNumberSource(SoundNumberSourceId),
}

pub(super) struct SoundNumberInputFocus {
    cursor: LexicalLayoutCursor,
    summon_widget_state: Option<SummonWidgetState<NumberSummonValue>>,
}

impl SoundNumberInputFocus {
    pub(super) fn new() -> SoundNumberInputFocus {
        SoundNumberInputFocus {
            cursor: LexicalLayoutCursor::new(),
            summon_widget_state: None,
        }
    }

    pub(super) fn cursor(&self) -> &LexicalLayoutCursor {
        &self.cursor
    }

    pub(super) fn cursor_mut(&mut self) -> &mut LexicalLayoutCursor {
        &mut self.cursor
    }

    pub(super) fn summon_widget_state_mut(
        &mut self,
    ) -> &mut Option<SummonWidgetState<NumberSummonValue>> {
        &mut self.summon_widget_state
    }
}

// TODO: add other presentations (e.g. plot, DAG maybe) and allow non-destructively switching between them
pub(super) struct SoundNumberInputPresentation {
    lexical_layout: LexicalLayout,
}

impl SoundNumberInputPresentation {
    pub(super) fn new(
        topology: &NumberGraphTopology,
        object_ui_states: &NumberObjectUiStates,
    ) -> SoundNumberInputPresentation {
        SoundNumberInputPresentation {
            lexical_layout: LexicalLayout::generate(topology, object_ui_states),
        }
    }

    pub(super) fn lexical_layout(&self) -> &LexicalLayout {
        &self.lexical_layout
    }

    pub(super) fn lexical_layout_mut(&mut self) -> &mut LexicalLayout {
        &mut self.lexical_layout
    }

    pub(super) fn cleanup(&mut self, topology: &NumberGraphTopology) {
        self.lexical_layout.cleanup(topology);
    }

    pub(super) fn handle_keypress(
        &mut self,
        ui: &egui::Ui,
        focus: &mut SoundNumberInputFocus,
        numberinputdata: &mut SoundNumberInputData,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
    ) {
        // TODO: combine available sound number sources and their names right here
        self.lexical_layout.handle_keypress(
            ui,
            focus,
            numberinputdata,
            object_factory,
            ui_factory,
            object_ui_states,
        )
    }
}

pub(super) struct SpatialGraphInputReference {
    input_id: NumberGraphInputId,
    location: egui::Pos2,
}

impl SpatialGraphInputReference {
    pub(super) fn new(
        input_id: NumberGraphInputId,
        location: egui::Pos2,
    ) -> SpatialGraphInputReference {
        SpatialGraphInputReference { input_id, location }
    }

    pub(super) fn input_id(&self) -> NumberGraphInputId {
        self.input_id
    }

    pub(super) fn location(&self) -> egui::Pos2 {
        self.location
    }

    pub(super) fn location_mut(&mut self) -> &mut egui::Pos2 {
        &mut self.location
    }
}

pub(super) struct SoundNumberInputUi {
    number_input_id: SoundNumberInputId,
}

impl SoundNumberInputUi {
    pub(super) fn new(id: SoundNumberInputId) -> SoundNumberInputUi {
        SoundNumberInputUi {
            number_input_id: id,
        }
    }

    pub(super) fn show(
        self,
        ui: &mut egui::Ui,
        result_label: &str,
        graph_state: &mut NumberGraphUiState,
        ctx: &NumberGraphUiContext,
        presentation: &mut SoundNumberInputPresentation,
        focus: Option<&mut SoundNumberInputFocus>,
    ) -> Vec<SpatialGraphInputReference> {
        // TODO: expandable/collapsible popup window with full layout
        let frame = egui::Frame::default()
            .fill(egui::Color32::BLACK)
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(64)))
            .inner_margin(egui::Margin::same(5.0));

        frame
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                presentation
                    .lexical_layout
                    .show(ui, result_label, graph_state, ctx, focus)
            })
            .inner
    }
}
