use std::{cell::RefCell, rc::Rc};

use eframe::egui;

use crate::core::{
    graph::{graphobject::GraphObjectHandle, objectfactory::ObjectFactory},
    jit::server::JitClient,
    number::numbergraph::NumberGraph,
    sound::{
        soundgraph::SoundGraph, soundgraphid::SoundObjectId, soundinput::SoundInputId,
        soundnumberinput::SoundNumberInputId,
    },
};

use super::{
    graph_ui::GraphUiContext,
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::{NumberGraphUiContext, OuterSoundNumberInputContext},
    soundgraphui::SoundGraphUi,
    soundgraphuinames::SoundGraphUiNames,
    soundgraphuistate::SoundGraphUiState,
    soundnumberinputui::SpatialGraphInputReference,
    soundobjectuistate::{AnySoundObjectUiData, SoundObjectUiStates},
    temporallayout::{TemporalLayout, TimeAxis},
    ui_factory::UiFactory,
};

pub struct SoundGraphUiContext<'a> {
    sound_ui_factory: &'a UiFactory<SoundGraphUi>,
    number_object_factory: &'a ObjectFactory<NumberGraph>,
    number_ui_factory: &'a UiFactory<NumberGraphUi>,
    sound_object_states: &'a SoundObjectUiStates,
    is_top_level: bool,
    time_axis: TimeAxis,
    width: f32,
    nesting_depth: usize,
    parent_input: Option<SoundInputId>,
    // TODO: encapsulate
    number_graph_input_references:
        Rc<RefCell<Vec<(SoundNumberInputId, Vec<SpatialGraphInputReference>)>>>,
    jit_client: &'a JitClient,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        ui_factory: &'a UiFactory<SoundGraphUi>,
        number_object_factory: &'a ObjectFactory<NumberGraph>,
        number_ui_factory: &'a UiFactory<NumberGraphUi>,
        object_states: &'a SoundObjectUiStates,
        is_top_level: bool,
        time_axis: TimeAxis,
        width: f32,
        nesting_depth: usize,
        jit_client: &'a JitClient,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            sound_ui_factory: ui_factory,
            number_object_factory,
            number_ui_factory,
            sound_object_states: object_states,
            is_top_level,
            time_axis,
            width,
            nesting_depth,
            parent_input: None,
            number_graph_input_references: Rc::new(RefCell::new(Vec::new())),
            jit_client,
        }
    }

    pub(crate) fn object_states(&self) -> &SoundObjectUiStates {
        self.sound_object_states
    }

    pub fn time_axis(&self) -> &TimeAxis {
        &self.time_axis
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub(super) fn number_graph_input_references(
        &self,
    ) -> &RefCell<Vec<(SoundNumberInputId, Vec<SpatialGraphInputReference>)>> {
        &*self.number_graph_input_references
    }

    pub fn is_top_level(&self) -> bool {
        self.is_top_level
    }

    pub(crate) fn show_nested_ui(
        &mut self,
        input_id: SoundInputId,
        desired_width: f32,
        target_graph_object: &GraphObjectHandle<SoundGraph>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        sound_graph: &mut SoundGraph,
    ) {
        let was_top_level = self.is_top_level;
        let old_width = self.width;
        let old_nesting_depth = self.nesting_depth;
        let old_parent_input = self.parent_input;

        self.is_top_level = false;
        self.width = desired_width;
        self.nesting_depth -= 1;
        self.parent_input = Some(input_id);

        self.sound_ui_factory
            .ui(target_graph_object, ui_state, ui, self, sound_graph);

        self.is_top_level = was_top_level;
        self.width = old_width;
        self.nesting_depth = old_nesting_depth;
        self.parent_input = old_parent_input;
    }

    pub(crate) fn nesting_depth(&self) -> usize {
        self.nesting_depth
    }

    pub(crate) fn parent_sound_input(&self) -> Option<SoundInputId> {
        self.parent_input
    }

    pub(crate) fn with_number_graph_ui_context<
        R,
        F: FnOnce(&mut NumberGraphUiContext, OuterSoundNumberInputContext) -> R,
    >(
        &mut self,
        input_id: SoundNumberInputId,
        temporal_layout: &TemporalLayout,
        names: &SoundGraphUiNames,
        sound_graph: &mut SoundGraph,
        f: F,
    ) -> R {
        let object_states = self.sound_object_states.number_graph_object_state(input_id);
        let owner = sound_graph
            .topology()
            .number_input(input_id)
            .unwrap()
            .owner();
        let sni_ctx = OuterSoundNumberInputContext::new(
            input_id,
            owner,
            temporal_layout,
            sound_graph,
            names,
            self.jit_client,
        );
        let mut ctx = NumberGraphUiContext::new(&self.number_ui_factory, object_states);
        f(&mut ctx, sni_ctx)
    }
}

impl<'a> GraphUiContext<'a> for SoundGraphUiContext<'a> {
    type GraphUi = SoundGraphUi;

    fn get_object_ui_data(&self, id: SoundObjectId) -> Rc<AnySoundObjectUiData> {
        self.sound_object_states.get_object_data(id)
    }
}
