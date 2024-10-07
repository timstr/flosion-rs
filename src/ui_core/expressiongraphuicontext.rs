use std::collections::HashSet;

use crate::core::{
    expression::expressiongraph::{
        ExpressionGraph, ExpressionGraphParameterId, ExpressionGraphResultId,
    },
    jit::cache::JitCache,
    sound::{
        expression::{ExpressionParameterMapping, ProcessorExpressionLocation},
        expressionargument::ArgumentLocation,
    },
};

use super::{
    expressionobjectui::ExpressionObjectUiFactory, soundgraphuinames::SoundGraphUiNames,
    stackedlayout::timeaxis::TimeAxis,
};

pub(crate) struct OuterProcessorExpressionContext<'a> {
    location: ProcessorExpressionLocation,
    parameter_mapping: &'a mut ExpressionParameterMapping,
    sound_graph_names: &'a SoundGraphUiNames,
    time_axis: TimeAxis,
    available_arguments: &'a HashSet<ArgumentLocation>,
}

impl<'a> OuterProcessorExpressionContext<'a> {
    pub(super) fn new(
        location: ProcessorExpressionLocation,
        parameter_mapping: &'a mut ExpressionParameterMapping,
        sound_graph_names: &'a SoundGraphUiNames,
        time_axis: TimeAxis,
        available_arguments: &'a HashSet<ArgumentLocation>,
    ) -> Self {
        Self {
            location,
            parameter_mapping,
            sound_graph_names,
            time_axis,
            available_arguments,
        }
    }

    pub(super) fn location(&self) -> ProcessorExpressionLocation {
        self.location
    }

    pub(super) fn mapping(&self) -> &ExpressionParameterMapping {
        self.parameter_mapping
    }

    pub(super) fn available_arguments(&self) -> &HashSet<ArgumentLocation> {
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
        argument_id: ArgumentLocation,
    ) -> Option<ExpressionGraphParameterId> {
        self.parameter_mapping.parameter_from_argument(argument_id)
    }

    pub(crate) fn connect_to_argument(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        argument_id: ArgumentLocation,
    ) -> ExpressionGraphParameterId {
        self.parameter_mapping
            .add_argument(argument_id, expression_graph)
    }

    pub(crate) fn disconnect_from_argument(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        argument_id: ArgumentLocation,
    ) {
        self.parameter_mapping
            .remove_argument(argument_id, expression_graph);
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
    pub(crate) fn parameter_name(&self, parameter_id: ExpressionGraphParameterId) -> String {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let nsid = ctx
                    .parameter_mapping
                    .argument_from_parameter(parameter_id)
                    .unwrap();
                ctx.sound_graph_names().combined_parameter_name(nsid)
            }
        }
    }

    pub(crate) fn result_name(&self, output_id: ExpressionGraphResultId) -> String {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => ctx
                .sound_graph_names()
                .expression(ctx.location())
                .unwrap()
                .name()
                .to_string(),
        }
    }

    pub(crate) fn remove_parameter(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        parameter_id: ExpressionGraphParameterId,
    ) {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                let arg_id = ctx
                    .parameter_mapping
                    .argument_from_parameter(parameter_id)
                    .unwrap();
                ctx.parameter_mapping
                    .remove_argument(arg_id, expression_graph);
            }
        }
    }
}

pub struct ExpressionGraphUiContext<'a, 'ctx> {
    ui_factory: &'a ExpressionObjectUiFactory,
    jit_cache: &'a JitCache<'ctx>,
}

impl<'a, 'ctx> ExpressionGraphUiContext<'a, 'ctx> {
    pub(super) fn new(
        ui_factory: &'a ExpressionObjectUiFactory,
        jit_cache: &'a JitCache<'ctx>,
    ) -> ExpressionGraphUiContext<'a, 'ctx> {
        ExpressionGraphUiContext {
            ui_factory,
            jit_cache,
        }
    }

    pub(super) fn ui_factory(&self) -> &ExpressionObjectUiFactory {
        self.ui_factory
    }

    pub(super) fn jit_cache(&self) -> &JitCache<'ctx> {
        self.jit_cache
    }
}
