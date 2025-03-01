use eframe::egui;

use crate::{
    core::{objecttype::ObjectType, sound::expression::ExpressionParameterTarget},
    ui_core::{
        expressiongraphuicontext::OuterProcessorExpressionContext,
        expressionobjectui::ExpressionObjectUiFactory,
        summon_widget::{SummonWidgetState, SummonWidgetStateBuilder},
    },
};

use super::ast::{VariableDefinition, VariableId};

#[derive(Copy, Clone)]
pub(super) enum ExpressionSummonValue {
    ExpressionNodeType(ObjectType),
    Constant(f32),
    ParameterTarget(ExpressionParameterTarget),
    Variable(VariableId),
}

pub(super) fn build_summon_widget_for_processor_expression(
    position: egui::Pos2,
    ui_factory: &ExpressionObjectUiFactory,
    ctx: &OuterProcessorExpressionContext,
    variable_definitions: &[VariableDefinition],
) -> SummonWidgetState<ExpressionSummonValue> {
    let mut builder = SummonWidgetStateBuilder::new(position);
    for object_ui in ui_factory.all_object_uis() {
        for name in object_ui.summon_names() {
            builder.add_name_with_arguments(
                name.to_string(),
                object_ui.summon_arguments(),
                ExpressionSummonValue::ExpressionNodeType(object_ui.object_type()),
            );
        }
    }

    builder.add_basic_name(
        "time".to_string(),
        ExpressionSummonValue::ParameterTarget(ExpressionParameterTarget::ProcessorTime(
            ctx.location().processor(),
        )),
    );

    builder.add_basic_name(
        format!(
            "{}.time",
            ctx.sound_graph_names()
                .sound_processor(ctx.location().processor())
                .unwrap()
        ),
        ExpressionSummonValue::ParameterTarget(ExpressionParameterTarget::ProcessorTime(
            ctx.location().processor(),
        )),
    );

    for input_loc in ctx.available_sound_inputs() {
        builder.add_basic_name(
            format!(
                "{}.time",
                ctx.sound_graph_names()
                    .sound_processor(input_loc.processor())
                    .unwrap()
            ),
            ExpressionSummonValue::ParameterTarget(ExpressionParameterTarget::ProcessorTime(
                input_loc.processor(),
            )),
        );

        builder.add_basic_name(
            format!(
                "{}.time",
                ctx.sound_graph_names().combined_input_name(*input_loc)
            ),
            ExpressionSummonValue::ParameterTarget(ExpressionParameterTarget::InputTime(
                *input_loc,
            )),
        );
    }

    for snsid in ctx.available_arguments() {
        builder.add_basic_name(
            ctx.sound_graph_names().combined_argument_name(*snsid),
            ExpressionSummonValue::ParameterTarget(ExpressionParameterTarget::Argument(*snsid)),
        );
    }

    for var_defn in variable_definitions {
        builder.add_basic_name(
            var_defn.name().to_string(),
            ExpressionSummonValue::Variable(var_defn.id()),
        );
    }

    // TODO: move this to the object ui after testing?
    builder.add_pattern("constant".to_string(), |s| {
        s.parse::<f32>()
            .ok()
            .and_then(|v| Some(ExpressionSummonValue::Constant(v)))
    });

    builder.build()
}
