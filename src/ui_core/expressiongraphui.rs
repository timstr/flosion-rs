use crate::core::expression::expressiongraph::ExpressionGraph;

use super::{
    expressiongraphuicontext::ExpressionGraphUiContext,
    expressiongraphuistate::AnyExpressionNodeObjectUiData, graph_ui::GraphUi,
};

pub struct ExpressionGraphUi {}

impl GraphUi for ExpressionGraphUi {
    type Graph = ExpressionGraph;

    // TODO: ???
    type State = ();

    type Context<'a> = ExpressionGraphUiContext<'a>;

    type ObjectUiData = AnyExpressionNodeObjectUiData;
}
