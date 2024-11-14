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
        expression::{
            ExpressionParameterMapping, ExpressionParameterTarget, ProcessorExpressionLocation,
        },
        soundinput::SoundInputLocation,
    },
};

use super::{
    expressionobjectui::ExpressionObjectUiFactory, history::SnapshotFlag,
    soundgraphuinames::SoundGraphUiNames, stackedlayout::timeaxis::TimeAxis,
};

pub(crate) struct OuterProcessorExpressionContext<'a> {
    location: ProcessorExpressionLocation,
    parameter_mapping: &'a mut ExpressionParameterMapping,
    sound_graph_names: &'a SoundGraphUiNames,
    time_axis: TimeAxis,
    available_sound_inputs: &'a HashSet<SoundInputLocation>,
    available_arguments: &'a HashSet<ProcessorArgumentLocation>,
    snapshot_flag: &'a SnapshotFlag,
}

impl<'a> OuterProcessorExpressionContext<'a> {
    pub(super) fn new(
        location: ProcessorExpressionLocation,
        parameter_mapping: &'a mut ExpressionParameterMapping,
        sound_graph_names: &'a SoundGraphUiNames,
        time_axis: TimeAxis,
        available_sound_inputs: &'a HashSet<SoundInputLocation>,
        available_arguments: &'a HashSet<ProcessorArgumentLocation>,
        snapshot_flag: &'a SnapshotFlag,
    ) -> Self {
        Self {
            location,
            parameter_mapping,
            sound_graph_names,
            time_axis,
            available_sound_inputs,
            available_arguments,
            snapshot_flag,
        }
    }

    pub(super) fn location(&self) -> ProcessorExpressionLocation {
        self.location
    }

    pub(super) fn mapping(&self) -> &ExpressionParameterMapping {
        self.parameter_mapping
    }

    pub(super) fn available_sound_inputs(&self) -> &HashSet<SoundInputLocation> {
        self.available_sound_inputs
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

    pub(crate) fn find_graph_id_for_target(
        &self,
        target: ExpressionParameterTarget,
    ) -> Option<ExpressionGraphParameterId> {
        self.parameter_mapping.parameter_from_target(target)
    }

    pub(crate) fn connect_to_target(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        target: ExpressionParameterTarget,
    ) -> ExpressionGraphParameterId {
        self.parameter_mapping.add_target(target, expression_graph)
    }

    pub(crate) fn disconnect_from_target(
        &mut self,
        expression_graph: &mut ExpressionGraph,
        target: ExpressionParameterTarget,
    ) {
        self.parameter_mapping
            .remove_target(target, expression_graph);
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
                let target = ctx
                    .parameter_mapping
                    .target_from_parameter(parameter_id)
                    .unwrap();
                let names = ctx.sound_graph_names();
                match target {
                    ExpressionParameterTarget::Argument(arg_loc) => {
                        names.combined_argument_name(arg_loc)
                    }
                    ExpressionParameterTarget::ProcessorTime(spid) => {
                        if spid == ctx.location.processor() {
                            "time".to_string()
                        } else {
                            format!("{}.time", names.sound_processor(spid).unwrap())
                        }
                    }
                    ExpressionParameterTarget::InputTime(input_loc) => {
                        format!("{}.time", names.combined_input_name(input_loc))
                    }
                }
            }
        }
    }

    pub(crate) fn result_name(&self, result_id: ExpressionInputId) -> &str {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => ctx
                .sound_graph_names()
                .expression_result(ctx.location(), result_id)
                .unwrap(),
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
                    .target_from_parameter(parameter_id)
                    .unwrap();
                ctx.parameter_mapping
                    .remove_target(arg_id, expression_graph);
            }
        }
    }

    pub(crate) fn request_snapshot(&self) {
        match self {
            OuterExpressionGraphUiContext::ProcessorExpression(ctx) => {
                ctx.snapshot_flag.request_snapshot()
            }
        }
    }
}

pub struct ExpressionGraphUiContext<'a, 'ctx> {
    object_factory: &'a ExpressionObjectFactory,
    ui_factory: &'a ExpressionObjectUiFactory,
    jit_cache: &'a JitCache<'ctx>,
    stash: &'a Stash,
    snapshot_flag: &'a SnapshotFlag,
}

impl<'a, 'ctx> ExpressionGraphUiContext<'a, 'ctx> {
    pub(super) fn new(
        object_factory: &'a ExpressionObjectFactory,
        ui_factory: &'a ExpressionObjectUiFactory,
        jit_cache: &'a JitCache<'ctx>,
        stash: &'a Stash,
        snapshot_flag: &'a SnapshotFlag,
    ) -> ExpressionGraphUiContext<'a, 'ctx> {
        ExpressionGraphUiContext {
            object_factory,
            ui_factory,
            jit_cache,
            stash,
            snapshot_flag,
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

    pub fn request_snapshot(&self) {
        self.snapshot_flag.request_snapshot();
    }
}
