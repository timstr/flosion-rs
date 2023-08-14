use std::cell::RefCell;

use crate::core::number::{numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId};

use super::{
    graph_ui::GraphUiContext,
    numbergraphui::NumberGraphUi,
    numbergraphuistate::{AnyNumberObjectUiData, NumberObjectUiStates},
    ui_factory::UiFactory,
};

pub struct NumberGraphUiContext<'a> {
    ui_factory: &'a UiFactory<NumberGraphUi>,
    object_states: &'a NumberObjectUiStates,
    topology: &'a NumberGraphTopology,
}

impl<'a> NumberGraphUiContext<'a> {
    pub(super) fn new(
        ui_factory: &'a UiFactory<NumberGraphUi>,
        object_states: &'a NumberObjectUiStates,
        topology: &'a NumberGraphTopology,
    ) -> NumberGraphUiContext<'a> {
        NumberGraphUiContext {
            ui_factory,
            object_states,
            topology,
        }
    }

    pub(super) fn topology(&self) -> &NumberGraphTopology {
        self.topology
    }
}

impl<'a> GraphUiContext<'a> for NumberGraphUiContext<'a> {
    type GraphUi = NumberGraphUi;

    fn get_object_ui_data(&self, id: NumberSourceId) -> &RefCell<AnyNumberObjectUiData> {
        todo!()
    }
}
