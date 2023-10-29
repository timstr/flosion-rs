use std::{any::Any, rc::Rc};

use serialization::Serializable;

use crate::core::graph::graph::Graph;

pub trait ObjectUiState: Any + Default + Serializable {}

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
    ) -> Rc<<Self::GraphUi as GraphUi>::ObjectUiData>;
}

pub trait ObjectUiData {
    type GraphUi: GraphUi;

    type RequiredData: Default + Serializable;

    fn new<S: ObjectUiState>(
        id: <<Self::GraphUi as GraphUi>::Graph as Graph>::ObjectId,
        state: S,
        data: Self::RequiredData,
    ) -> Self;

    type ConcreteType<'a, T: ObjectUiState>;

    fn downcast_with<
        T: ObjectUiState,
        F: FnOnce(
            Self::ConcreteType<'_, T>,
            &mut <Self::GraphUi as GraphUi>::State,
            &mut <Self::GraphUi as GraphUi>::Context<'_>,
        ),
    >(
        &self,
        ui_state: &mut <Self::GraphUi as GraphUi>::State,
        ctx: &mut <Self::GraphUi as GraphUi>::Context<'_>,
        f: F,
    );
}
