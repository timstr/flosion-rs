use std::{any::Any, cell::RefCell, rc::Rc};

use crate::core::graph::graph::Graph;

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

    /// Per-object data that is needed before the object is rendered
    /// in order to display it properly
    type Properties;
}

pub trait GraphUiState {
    type GraphUi: GraphUi;

    /// Access the ui data of an individual graph object by its id.
    /// This is used in UiFactory to automate rendering the ui of
    /// any given object.
    fn get_object_ui_data(
        &self,
        id: <<Self::GraphUi as GraphUi>::Graph as Graph>::ObjectId,
    ) -> Rc<RefCell<dyn Any>>;
}
