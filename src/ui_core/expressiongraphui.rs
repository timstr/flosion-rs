use crate::core::expression::expressiongraph::ExpressionGraph;

use super::{
    expressiongraphuicontext::ExpressionGraphUiContext, graph_ui::GraphUi,
    lexicallayout::lexicallayout::ExpressionNodeLayout,
};

// TODO: delete
pub struct ExpressionGraphUi {}

// TODO: delete
impl GraphUi for ExpressionGraphUi {
    type Graph = ExpressionGraph;

    type Context<'a> = ExpressionGraphUiContext<'a>;

    type Properties = ExpressionNodeLayout;
}
