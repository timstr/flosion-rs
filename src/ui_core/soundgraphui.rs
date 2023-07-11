use crate::core::sound::soundgraph::SoundGraph;

use super::{
    graph_ui::GraphUi, soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUIState, soundobjectuistate::AnySoundObjectUiData,
};

pub struct SoundGraphUi {}

impl GraphUi for SoundGraphUi {
    type Graph = SoundGraph;

    type State = SoundGraphUIState;

    type Context<'a> = SoundGraphUiContext<'a>;

    type ObjectUiData = AnySoundObjectUiData;
}
