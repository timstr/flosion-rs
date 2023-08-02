use std::cell::RefCell;

use crate::core::graph::graph::Graph;

use super::object_ui::ObjectUiState;

pub trait GraphUi {
    // the graph type being represented in the ui
    type Graph: Graph;

    // graph-wide ui state
    type State;

    // extra data passed to each individual object ui
    type Context<'a>: GraphUiContext<'a, GraphUi = Self>;

    // data associated with individual object ui
    type ObjectUiData: ObjectUiData<GraphUi = Self>;
}

pub trait GraphUiContext<'a> {
    type GraphUi: GraphUi;

    fn get_object_ui_data(
        &self,
        id: <<Self::GraphUi as GraphUi>::Graph as Graph>::ObjectId,
    ) -> &RefCell<<Self::GraphUi as GraphUi>::ObjectUiData>;
}

pub trait ObjectUiData {
    type GraphUi: GraphUi;

    type ConcreteType<'a, T: ObjectUiState>
    where
        Self: 'a;

    fn downcast<'a, T: ObjectUiState>(
        &'a mut self,
        ui_state: &<Self::GraphUi as GraphUi>::State,
        ctx: &<Self::GraphUi as GraphUi>::Context<'_>,
    ) -> Self::ConcreteType<'a, T>;
}
