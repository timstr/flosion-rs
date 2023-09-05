use std::{cell::RefCell, rc::Rc};

use crate::core::sound::{
    soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
};

use super::{
    graph_ui::GraphUiContext,
    numbergraphui::NumberGraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    soundgraphui::SoundGraphUi,
    soundnumberinputui::SpatialGraphInputReference,
    soundobjectuistate::{AnySoundObjectUiData, SoundObjectUiStates},
    temporallayout::TimeAxis,
    ui_factory::UiFactory,
};

pub struct SoundGraphUiContext<'a> {
    // TODO: rename ui_factory to sound_ui_factory
    ui_factory: &'a UiFactory<SoundGraphUi>,
    number_ui_factory: &'a UiFactory<NumberGraphUi>,
    // TODO: rename object_states to sound_object_states
    object_states: &'a SoundObjectUiStates,
    topology: &'a SoundGraphTopology,
    is_top_level: bool,
    time_axis: TimeAxis,
    width: f32,
    nesting_depth: usize,
    parent_input: Option<SoundInputId>,
    number_graph_input_references:
        Rc<RefCell<Vec<(SoundNumberInputId, Vec<SpatialGraphInputReference>)>>>,
}

impl<'a> SoundGraphUiContext<'a> {
    pub(crate) fn new(
        ui_factory: &'a UiFactory<SoundGraphUi>,
        number_ui_factory: &'a UiFactory<NumberGraphUi>,
        object_states: &'a SoundObjectUiStates,
        topology: &'a SoundGraphTopology,
        is_top_level: bool,
        time_axis: TimeAxis,
        width: f32,
        nesting_depth: usize,
    ) -> SoundGraphUiContext<'a> {
        SoundGraphUiContext {
            ui_factory,
            number_ui_factory,
            object_states,
            topology,
            is_top_level,
            time_axis,
            width,
            nesting_depth,
            parent_input: None,
            number_graph_input_references: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub(crate) fn ui_factory(&self) -> &UiFactory<SoundGraphUi> {
        self.ui_factory
    }

    pub(crate) fn object_states(&self) -> &SoundObjectUiStates {
        self.object_states
    }

    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        self.topology
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

    pub(crate) fn nest(&self, input_id: SoundInputId, new_width: f32) -> SoundGraphUiContext {
        SoundGraphUiContext {
            ui_factory: self.ui_factory,
            number_ui_factory: self.number_ui_factory,
            object_states: self.object_states,
            topology: self.topology,
            is_top_level: false,
            time_axis: self.time_axis,
            width: new_width,
            nesting_depth: self.nesting_depth - 1,
            parent_input: Some(input_id),
            number_graph_input_references: Rc::clone(&self.number_graph_input_references),
        }
    }

    pub(crate) fn nesting_depth(&self) -> usize {
        self.nesting_depth
    }

    pub(crate) fn parent_sound_input(&self) -> Option<SoundInputId> {
        self.parent_input
    }

    pub(crate) fn number_graph_ui_context(
        &self,
        input_id: SoundNumberInputId,
    ) -> NumberGraphUiContext {
        let object_states = self.object_states.number_graph_object_state(input_id);
        let topology = self
            .topology
            .number_input(input_id)
            .unwrap()
            .number_graph()
            .topology();
        NumberGraphUiContext::new(&self.number_ui_factory, object_states, topology)
    }
}

impl<'a> GraphUiContext<'a> for SoundGraphUiContext<'a> {
    type GraphUi = SoundGraphUi;

    fn get_object_ui_data(&self, id: SoundObjectId) -> &AnySoundObjectUiData {
        self.object_states.get_object_data(id)
    }
}
