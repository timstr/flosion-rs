use crate::core::graph::graph::Graph;

// TODO Uhhhhhhhhhh I kind of hate this
// Being able to reuse things like UiFactory and ObjectUi is nice buttt
// ideally I wouldn't need to constrain the living heck out of both
// expression and sound graph UIs.
pub trait GraphUi {
    /// the graph type being represented in the ui
    type Graph: Graph;

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
