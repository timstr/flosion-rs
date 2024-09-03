use std::collections::HashSet;

use crate::core::{
    expression::expressiongraph::{
        ExpressionGraph, ExpressionGraphParameterId, ExpressionGraphResultId,
    },
    jit::server::JitServer,
    sound::{
        expression::SoundExpressionId, expressionargument::SoundExpressionArgumentId,
        sounderror::SoundError, soundgraph::SoundGraph, soundgraphtopology::SoundGraphTopology,
    },
};

use super::{
    expressionobjectui::ExpressionObjectUiFactory, soundgraphuinames::SoundGraphUiNames,
    stackedlayout::timeaxis::TimeAxis,
};

#[derive(Clone, Copy)]
pub(crate) struct OuterProcessorExpressionContext<'a> {
    expression_id: SoundExpressionId,
    sound_graph_names: &'a SoundGraphUiNames,
    time_axis: TimeAxis,
    available_arguments: &'a HashSet<SoundExpressionArgumentId>,
}

impl<'a> OuterProcessorExpressionContext<'a> {
    pub(super) fn new(
        expression_id: SoundExpressionId,
        sound_graph_names: &'a SoundGraphUiNames,
        time_axis: TimeAxis,
        available_arguments: &'a HashSet<SoundExpressionArgumentId>,
    ) -> Self {
        Self {
            expression_id,
            sound_graph_names,
            time_axis,
            available_arguments,
        }
    }

    pub(super) fn expression_id(&self) -> SoundExpressionId {
        self.expression_id
    }

    pub(super) fn available_arguments(&self) -> &HashSet<SoundExpressionArgumentId> {
        self.available_arguments
    }

    pub(crate) fn sound_graph_names(&self) -> &SoundGraphUiNames {
        self.sound_graph_names
    }

    pub(crate) fn time_axis(&self) -> &TimeAxis {
        &self.time_axis
    }

    pub(crate) fn find_graph_id_for_argument(
        &self,
        topology: &SoundGraphTopology,
        argument_id: SoundExpressionArgumentId,
    ) -> Option<ExpressionGraphParameterId> {
        topology
            .expression(self.expression_id)
            .unwrap()
            .parameter_mapping()
            .parameter_from_argument(argument_id)
    }

    pub(crate) fn connect_to_argument(
        &self,
        sound_graph: &mut SoundGraph,
        argument_id: SoundExpressionArgumentId,
    ) -> ExpressionGraphParameterId {
        sound_graph
            .edit_expression(self.expression_id, |ni_data| {
                let (expr_graph, mapping) = ni_data.expression_graph_and_mapping_mut();
                mapping.add_argument(argument_id, expr_graph)
            })
            .unwrap()
    }

    pub(crate) fn disconnect_from_argument(
        &self,
        sound_graph: &mut SoundGraph,
        nsid: SoundExpressionArgumentId,
    ) {
        sound_graph
            .edit_expression(self.expression_id, |ni_data| {
                let (expr_graph, mapping) = ni_data.expression_graph_and_mapping_mut();
                mapping.remove_argument(nsid, expr_graph);
            })
            .unwrap();
    }
}

#[derive(Clone, Copy)]
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
    pub(crate) fn parameter_name(
        &self,
        topology: &SoundGraphTopology,
        input_id: ExpressionGraphParameterId,
    ) -> String {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let nsid = topology
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
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => ctx
                .sound_graph_names()
                .expression(ctx.expression_id())
                .unwrap()
                .name()
                .to_string(),
        }
    }

    pub(crate) fn inspect_expression_graph<R, F: FnOnce(&ExpressionGraph) -> R>(
        &self,
        topology: &SoundGraphTopology,
        f: F,
    ) -> R {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => f(topology
                .expression(ctx.expression_id())
                .unwrap()
                .expression_graph()),
        }
    }

    pub(crate) fn edit_expression_graph<R, F: FnOnce(&mut ExpressionGraph) -> R>(
        &self,
        sound_graph: &mut SoundGraph,
        f: F,
    ) -> Result<R, SoundError> {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let niid = ctx.expression_id();
                sound_graph.edit_expression(niid, |ni_data| f(ni_data.expression_graph_mut()))
            }
        }
    }

    pub(crate) fn remove_parameter(
        &self,
        sound_graph: &mut SoundGraph,
        giid: ExpressionGraphParameterId,
    ) {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let niid = ctx.expression_id();
                sound_graph
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

pub struct ExpressionGraphUiContext<'a, 'ctx> {
    ui_factory: &'a ExpressionObjectUiFactory,
    jit_server: &'a JitServer<'ctx>,
}

impl<'a, 'ctx> ExpressionGraphUiContext<'a, 'ctx> {
    pub(super) fn new(
        ui_factory: &'a ExpressionObjectUiFactory,
        jit_server: &'a JitServer<'ctx>,
    ) -> ExpressionGraphUiContext<'a, 'ctx> {
        ExpressionGraphUiContext {
            ui_factory,
            jit_server,
        }
    }

    pub(super) fn ui_factory(&self) -> &ExpressionObjectUiFactory {
        self.ui_factory
    }

    pub(super) fn jit_server(&self) -> &JitServer<'ctx> {
        self.jit_server
    }
}
