use std::collections::HashSet;

use crate::core::{
    expression::expressiongraph::{
        ExpressionGraph, ExpressionGraphParameterId, ExpressionGraphResultId,
    },
    sound::{
        expression::SoundExpressionId, expressionargument::SoundExpressionArgumentId,
        soundgraph::SoundGraph, sounderror::SoundError, soundprocessor::SoundProcessorId,
    },
};

use super::{
    expressiongraphui::ExpressionGraphUi, soundgraphuinames::SoundGraphUiNames,
    stackedlayout::timeaxis::TimeAxis, ui_factory::UiFactory,
};

pub(crate) struct OuterProcessorExpressionContext<'a> {
    expression_id: SoundExpressionId,
    parent_sound_processor_id: SoundProcessorId,
    sound_graph: &'a mut SoundGraph,
    sound_graph_names: &'a SoundGraphUiNames,
    time_axis: TimeAxis,
    available_arguments: &'a HashSet<SoundExpressionArgumentId>,
}

impl<'a> OuterProcessorExpressionContext<'a> {
    pub(super) fn new(
        expression_id: SoundExpressionId,
        parent_sound_processor_id: SoundProcessorId,
        sound_graph: &'a mut SoundGraph,
        sound_graph_names: &'a SoundGraphUiNames,
        time_axis: TimeAxis,
        available_arguments: &'a HashSet<SoundExpressionArgumentId>,
    ) -> Self {
        Self {
            expression_id,
            parent_sound_processor_id,
            sound_graph,
            sound_graph_names,
            time_axis,
            available_arguments,
        }
    }

    pub(super) fn expression_id(&self) -> SoundExpressionId {
        self.expression_id
    }

    pub(super) fn parent_sound_processor_id(&self) -> SoundProcessorId {
        self.parent_sound_processor_id
    }

    pub(super) fn available_arguments(&self) -> &HashSet<SoundExpressionArgumentId> {
        self.available_arguments
    }

    pub(crate) fn sound_graph(&self) -> &SoundGraph {
        self.sound_graph
    }

    pub(crate) fn sound_graph_mut(&mut self) -> &mut SoundGraph {
        self.sound_graph
    }

    pub(crate) fn sound_graph_names(&self) -> &SoundGraphUiNames {
        self.sound_graph_names
    }

    pub(crate) fn time_axis(&self) -> &TimeAxis {
        &self.time_axis
    }

    pub(crate) fn find_graph_id_for_argument(
        &self,
        argument_id: SoundExpressionArgumentId,
    ) -> Option<ExpressionGraphParameterId> {
        self.sound_graph
            .topology()
            .expression(self.expression_id)
            .unwrap()
            .parameter_mapping()
            .parameter_from_argument(argument_id)
    }

    pub(crate) fn connect_to_argument(
        &mut self,
        argument_id: SoundExpressionArgumentId,
    ) -> ExpressionGraphParameterId {
        self.sound_graph
            .edit_expression(self.expression_id, |ni_data| {
                let (expr_graph, mapping) = ni_data.expression_graph_and_mapping_mut();
                mapping.add_argument(argument_id, expr_graph)
            })
            .unwrap()
    }

    pub(crate) fn disconnect_from_argument(&mut self, nsid: SoundExpressionArgumentId) {
        self.sound_graph
            .edit_expression(self.expression_id, |ni_data| {
                let (expr_graph, mapping) = ni_data.expression_graph_and_mapping_mut();
                mapping.remove_argument(nsid, expr_graph);
            })
            .unwrap();
    }
}

pub(crate) enum OuterExpressionGraphUiContext<'a> {
    // TODO: top level expression graph/function also
    ProcessorExpression(OuterProcessorExpressionContext<'a>),
}

impl<'a> From<OuterProcessorExpressionContext<'a>> for OuterExpressionGraphUiContext<'a> {
    fn from(value: OuterProcessorExpressionContext<'a>) -> Self {
        OuterExpressionGraphUiContext::ProcessorExpression(value)
    }
}

impl<'a> OuterExpressionGraphUiContext<'a> {
    pub(crate) fn parameter_name(&self, input_id: ExpressionGraphParameterId) -> String {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let nsid = ctx
                    .sound_graph()
                    .topology()
                    .expression(ctx.expression_id())
                    .unwrap()
                    .parameter_mapping()
                    .argument_from_parameter(input_id)
                    .unwrap();
                ctx.sound_graph_names().combined_parameter_name(nsid)
            }
        }
    }

    pub(crate) fn result_name(&self, output_id: ExpressionGraphResultId) -> String {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                assert!(self.inspect_expression_graph(|g| {
                    let outputs = g.topology().results();
                    assert_eq!(outputs.len(), 1);
                    outputs[0].id() == output_id
                }));
                ctx.sound_graph_names()
                    .expression(ctx.expression_id())
                    .unwrap()
                    .name()
                    .to_string()
            }
        }
    }

    pub(crate) fn inspect_expression_graph<R, F: FnOnce(&ExpressionGraph) -> R>(&self, f: F) -> R {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => f(ctx
                .sound_graph()
                .topology()
                .expression(ctx.expression_id())
                .unwrap()
                .expression_graph()),
        }
    }

    pub(crate) fn edit_expression_graph<R, F: FnOnce(&mut ExpressionGraph) -> R>(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let niid = ctx.expression_id();
                ctx.sound_graph_mut()
                    .edit_expression(niid, |ni_data| f(ni_data.expression_graph_mut()))
            }
        }
    }

    pub(crate) fn remove_parameter(&mut self, giid: ExpressionGraphParameterId) {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let niid = ctx.expression_id();
                ctx.sound_graph_mut()
                    .edit_expression(niid, |ni_data| {
                        let (expr_graph, mapping) = ni_data.expression_graph_and_mapping_mut();
                        let argument_id = mapping.argument_from_parameter(giid).unwrap();
                        mapping.remove_argument(argument_id, expr_graph);
                    })
                    .unwrap();
            }
        }
    }
}

pub struct ExpressionGraphUiContext<'a> {
    ui_factory: &'a UiFactory<ExpressionGraphUi>,
}

impl<'a> ExpressionGraphUiContext<'a> {
    pub(super) fn new(
        ui_factory: &'a UiFactory<ExpressionGraphUi>,
    ) -> ExpressionGraphUiContext<'a> {
        ExpressionGraphUiContext { ui_factory }
    }

    pub(super) fn ui_factory(&self) -> &UiFactory<ExpressionGraphUi> {
        self.ui_factory
    }
}
