use crate::core::expression::expressiongraph::ExpressionGraph;

use super::{
    expressiongraphuicontext::ExpressionGraphUiContext,
    expressiongraphuistate::ExpressionGraphUiState, graph_ui::GraphUi,
    lexicallayout::lexicallayout::ExpressionNodeLayout,
};

// TODO: delete
pub struct ExpressionGraphUi {}

// TODO: delete
impl GraphUi for ExpressionGraphUi {
    type Graph = ExpressionGraph;

    type State = ExpressionGraphUiState;

    type Context<'a> = ExpressionGraphUiContext<'a>;

    type Properties = ExpressionNodeLayout;
}
