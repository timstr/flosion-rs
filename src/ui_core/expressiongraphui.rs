use crate::core::expression::expressiongraph::ExpressionGraph;

use super::{
    expressiongraphuicontext::ExpressionGraphUiContext,
    expressiongraphuistate::ExpressionGraphUiState, graph_ui::GraphUi,
    lexicallayout::lexicallayout::ExpressionNodeLayout,
};

pub struct ExpressionGraphUi {}

impl GraphUi for ExpressionGraphUi {
    type Graph = ExpressionGraph;

    type State = ExpressionGraphUiState;

    type Context<'a> = ExpressionGraphUiContext<'a>;

    type Properties = ExpressionNodeLayout;
}
