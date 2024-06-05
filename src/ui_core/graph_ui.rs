use std::{any::Any, rc::Rc};

use serialization::Serializable;

use crate::core::graph::graph::Graph;

pub trait ObjectUiState: Any + Serializable {}

pub trait GraphUi {
    /// the graph type being represented in the ui
    type Graph: Graph;

    /// graph-wide ui state, containing per-object
    /// ui data. See also GraphUiState below.
    type State: GraphUiState<GraphUi = Self>;

    /// extra data passed to each individual object ui.
    // NOTE: this lifetime is here because both
    // SoundGraphUiContext and ExpressionGraphUiContext
    // require lifetimes, and there doesn't seem to be
    // a way to elide it currently.
    type Context<'a>;

    /// data associated with individual object ui
    type ObjectUiData: ObjectUiData<GraphUi = Self>;
}

pub trait GraphUiState {
    type GraphUi: GraphUi;

    /// Access the ui data of an individual graph object by its id.
    /// This is used in UiFactory to automate rendering the ui of
    /// any given object.
    fn get_object_ui_data(
        &self,
        id: <<Self::GraphUi as GraphUi>::Graph as Graph>::ObjectId,
    ) -> Rc<<Self::GraphUi as GraphUi>::ObjectUiData>;
}

pub trait ObjectUiData {
    type GraphUi: GraphUi;

    // TODO: think of a better name
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
            &<Self::GraphUi as GraphUi>::Context<'_>,
        ),
    >(
        &self,
        ui_state: &mut <Self::GraphUi as GraphUi>::State,
        ctx: &<Self::GraphUi as GraphUi>::Context<'_>,
        f: F,
    );
}
