use crate::core::number::numbergraph::NumberGraph;

use super::{
    graph_ui::GraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{AnyNumberObjectUiData, NumberGraphUiState},
};

pub struct NumberGraphUi {}

impl GraphUi for NumberGraphUi {
    type Graph = NumberGraph;

    type State = NumberGraphUiState;

    type Context<'a> = NumberGraphUiContext<'a>;

    type ObjectUiData = AnyNumberObjectUiData;
}
