use crate::core::expression::expressiongraph::ExpressionGraph;

use super::{
    graph_ui::GraphUi,
    numbergraphuicontext::NumberGraphUiContext,
    numbergraphuistate::{AnyNumberObjectUiData, NumberGraphUiState},
};

pub struct NumberGraphUi {}

impl GraphUi for NumberGraphUi {
    type Graph = ExpressionGraph;

    type State = NumberGraphUiState;

    type Context<'a> = NumberGraphUiContext<'a>;

    type ObjectUiData = AnyNumberObjectUiData;
}
