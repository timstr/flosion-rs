use crate::core::sound::soundgraph::SoundGraph;

use super::{
    graph_ui::GraphUi, soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState, soundobjectuistate::AnySoundObjectUiData,
};

pub struct SoundGraphUi {}

impl GraphUi for SoundGraphUi {
    type Graph = SoundGraph;

    type State = SoundGraphUiState;

    type Context<'a> = SoundGraphUiContext<'a>;

    type ObjectUiData = AnySoundObjectUiData;
}
