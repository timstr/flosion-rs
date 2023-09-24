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
    lexicallayout::lexicallayout::{LexicalLayout, LexicalLayoutFocus},
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{NumberGraphUiState, NumberObjectUiStates},
    temporallayout::TemporalLayout,
    ui_factory::UiFactory,
};

#[derive(Copy, Clone)]
pub(super) enum NumberSummonValue {
    NumberSourceType(ObjectType),
    SoundNumberSource(SoundNumberSourceId),
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
        focus: &mut LexicalLayoutFocus,
        numberinputdata: &mut SoundNumberInputData,
        object_factory: &ObjectFactory<NumberGraph>,
        ui_factory: &UiFactory<NumberGraphUi>,
        object_ui_states: &mut NumberObjectUiStates,
        temporal_layout: &TemporalLayout,
    ) {
        // TODO: combine available sound number sources and their names right here
        // - The lexical layout should accept a list of custom entries with names
        //   and functions to call as well as additions to the lexical layout
        //    -> for example, selecting a sound number source should:
        //       1. edit the numberinputdata to add the target number source
        //          and obtain its graph input id
        //    -> 2. produce an ASTNode pointing to the graph input with its
        //          human-readable name
        //           -> !!!!!!!!!!!!! N.B. the summon widget is not bespoke to
        //              lexical layout, so this ASTNode will need to be
        //              through some other means. Can this be done within
        //              LexicalLayout while keeping SummonWidget use agnostic?
        // - This requires decoupling the numberinputdata's numbergraph from
        //   the input mapping because the LexicalLayout holds a mutable
        //   reference to the numbergraph inside the following function, and
        //   so the callback function involved here can't also hold a mutable
        //   reference to the numberinputdata containing the numbergraph.
        //    -> This conflict can be resolved by having the callback accept
        //       a mutable reference to the numbergraph which is provided
        //       by the LexicalLayout
        // - What's more tricky (at first glance) is that the LexicalLayoutFocus
        //   stores a list of displayed entries, but doing this with the callback
        //   functions stored here would effectively require them to hold no
        //   references which would make them nearly useless.
        //    -> Instead of storing each function, the following function could
        //       simply take a list of callback functions with a name uniquely
        //       assigned to each, and store these names only. When a name is
        //       chosen, it makes no difference that the callback has been
        //       reconstructed many times since the summon widget was opened,
        //       since it will only be called once
        // ***
        // Ok, the above design seems viable, safe, and easy enough to implement.
        // What problems would it help solve in other areas?
        // Summon widget at top level:
        //  - TODO
        // Summon widget for sound inputs:
        //  - TODO
        self.lexical_layout.handle_keypress(
            ui,
            focus,
            numberinputdata.number_graph_mut(),
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
        focus: Option<&mut LexicalLayoutFocus>,
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
            .inner;

        // TODO: consider traversing the lexical layout in search of graph inputs
        Vec::new()
    }
}
