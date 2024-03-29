use eframe::egui;

use crate::{
    core::{graph::graphobject::ObjectType, sound::soundnumbersource::SoundNumberSourceId},
    ui_core::{
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::OuterSoundNumberInputContext,
        summon_widget::{SummonWidgetState, SummonWidgetStateBuilder},
        ui_factory::UiFactory,
    },
};

use super::ast::{VariableDefinition, VariableId};

#[derive(Copy, Clone)]
pub(super) enum NumberSummonValue {
    NumberSourceType(ObjectType),
    Constant(f32),
    SoundNumberSource(SoundNumberSourceId),
    Variable(VariableId),
}

pub(super) fn build_summon_widget_for_sound_number_input(
    position: egui::Pos2,
    ui_factory: &UiFactory<NumberGraphUi>,
    ctx: &OuterSoundNumberInputContext,
    variable_definitions: &[VariableDefinition],
) -> SummonWidgetState<NumberSummonValue> {
    let mut builder = SummonWidgetStateBuilder::new(position);
    for object_ui in ui_factory.all_object_uis() {
        for name in object_ui.summon_names() {
            builder.add_name_with_arguments(
                name.to_string(),
                object_ui.summon_arguments(),
                NumberSummonValue::NumberSourceType(object_ui.object_type()),
            );
        }
    }

    for snsid in ctx
        .temporal_layout()
        .available_number_sources(ctx.sound_number_input_id())
    {
        builder.add_basic_name(
            ctx.sound_graph_names().combined_number_source_name(*snsid),
            NumberSummonValue::SoundNumberSource(*snsid),
        );
    }

    for var_defn in variable_definitions {
        builder.add_basic_name(
            var_defn.name().to_string(),
            NumberSummonValue::Variable(var_defn.id()),
        );
    }

    // TODO: move this to the object ui after testing?
    builder.add_pattern("constant".to_string(), |s| {
        s.parse::<f32>()
            .ok()
            .and_then(|v| Some(NumberSummonValue::Constant(v)))
    });

    builder.build()
}
