use std::collections::HashSet;

use hashstash::Stash;

use crate::core::{
    expression::{
        expressiongraph::{ExpressionGraph, ExpressionGraphParameterId},
        expressioninput::ExpressionInputId,
        expressionobject::ExpressionObjectFactory,
    },
    jit::cache::JitCache,
    sound::{
        argument::ProcessorArgumentLocation,
        expression::{ExpressionParameterMapping, ProcessorExpressionLocation},
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
    available_arguments: &'a HashSet<ProcessorArgumentLocation>,
}

impl<'a> OuterProcessorExpressionContext<'a> {
    pub(super) fn new(
        location: ProcessorExpressionLocation,
        parameter_mapping: &'a mut ExpressionParameterMapping,
        sound_graph_names: &'a SoundGraphUiNames,
        time_axis: TimeAxis,
        available_arguments: &'a HashSet<ProcessorArgumentLocation>,
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

    pub(super) fn available_arguments(&self) -> &HashSet<ProcessorArgumentLocation> {
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
        argument_id: ProcessorArgumentLocation,
    ) -> Option<ExpressionGraphParameterId> {
        self.parameter_mapping.parameter_from_argument(argument_id)
    }

    pub(crate) fn connect_to_argument(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        argument_id: ProcessorArgumentLocation,
    ) -> ExpressionGraphParameterId {
        self.parameter_mapping
            .add_argument(argument_id, expression_graph)
    }

    pub(crate) fn disconnect_from_argument(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        argument_id: ProcessorArgumentLocation,
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

    pub(crate) fn result_name(&self, result_id: ExpressionInputId) -> &str {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                ctx.sound_graph_names().expression(ctx.location()).unwrap()
            }
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
    object_factory: &'a ExpressionObjectFactory,
    ui_factory: &'a ExpressionObjectUiFactory,
    jit_cache: &'a JitCache<'ctx>,
    stash: &'a Stash,
}

impl<'a, 'ctx> ExpressionGraphUiContext<'a, 'ctx> {
    pub(super) fn new(
        object_factory: &'a ExpressionObjectFactory,
        ui_factory: &'a ExpressionObjectUiFactory,
        jit_cache: &'a JitCache<'ctx>,
        stash: &'a Stash,
    ) -> ExpressionGraphUiContext<'a, 'ctx> {
        ExpressionGraphUiContext {
            object_factory,
            ui_factory,
            jit_cache,
            stash,
        }
    }

    pub(super) fn object_factory(&self) -> &ExpressionObjectFactory {
        self.object_factory
    }

    pub(super) fn ui_factory(&self) -> &ExpressionObjectUiFactory {
        self.ui_factory
    }

    pub(super) fn jit_cache(&self) -> &JitCache<'ctx> {
        self.jit_cache
    }

    pub(super) fn stash(&self) -> &Stash {
        self.stash
    }
}
