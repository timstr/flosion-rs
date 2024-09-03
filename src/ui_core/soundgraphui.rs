use crate::core::sound::soundgraph::SoundGraph;

use super::{graph_ui::GraphUi, soundgraphuicontext::SoundGraphUiContext};

pub struct SoundGraphUi {}

impl GraphUi for SoundGraphUi {
    type Graph = SoundGraph;

    type Context<'a> = SoundGraphUiContext<'a>;

    type Properties = ();
}
