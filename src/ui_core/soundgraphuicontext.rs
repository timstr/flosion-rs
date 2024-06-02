use std::rc::Rc;

use eframe::egui;

use crate::core::{
    expression::expressiongraph::ExpressionGraph,
    graph::{graphobject::GraphObjectHandle, objectfactory::ObjectFactory},
    jit::server::JitClient,
    sound::{soundgraph::SoundGraph, soundgraphid::SoundObjectId, soundinput::SoundInputId},
};

use super::{
    expressiongraphui::ExpressionGraphUi,
    graph_ui::GraphUiContext,
    soundgraphlayout::TimeAxis,
    soundgraphui::SoundGraphUi,
    soundgraphuistate::SoundGraphUiState,
    soundobjectuistate::{AnySoundObjectUiData, SoundObjectUiStates},
    ui_factory::UiFactory,
};

pub struct SoundGraphUiContext<'a> {
    sound_ui_factory: &'a UiFactory<SoundGraphUi>,
    _expression_object_factory: &'a ObjectFactory<ExpressionGraph>,
    expression_ui_factory: &'a UiFactory<ExpressionGraphUi>,
    sound_object_states: &'a SoundObjectUiStates,
    is_top_level: bool,
    time_axis: TimeAxis,
    width: f32,
    parent_input: Option<SoundInputId>,
    jit_client: &'a JitClient,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        ui_factory: &'a UiFactory<SoundGraphUi>,
        expression_object_factory: &'a ObjectFactory<ExpressionGraph>,
        expression_ui_factory: &'a UiFactory<ExpressionGraphUi>,
        object_states: &'a SoundObjectUiStates,
        is_top_level: bool,
        time_axis: TimeAxis,
        width: f32,
        jit_client: &'a JitClient,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            sound_ui_factory: ui_factory,
            _expression_object_factory: expression_object_factory,
            expression_ui_factory,
            sound_object_states: object_states,
            is_top_level,
            time_axis,
            width,
            parent_input: None,
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
        let old_parent_input = self.parent_input;

        self.is_top_level = false;
        self.width = desired_width;
        self.parent_input = Some(input_id);

        self.sound_ui_factory
            .ui(target_graph_object, ui_state, ui, self, sound_graph);

        self.is_top_level = was_top_level;
        self.width = old_width;
        self.parent_input = old_parent_input;
    }

    pub(crate) fn parent_sound_input(&self) -> Option<SoundInputId> {
        self.parent_input
    }
}

impl<'a> GraphUiContext<'a> for SoundGraphUiContext<'a> {
    type GraphUi = SoundGraphUi;

    fn get_object_ui_data(&self, id: SoundObjectId) -> Rc<AnySoundObjectUiData> {
        self.sound_object_states.get_object_data(id)
    }
}
