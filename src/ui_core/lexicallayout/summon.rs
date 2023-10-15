use eframe::egui;

use crate::{
    core::{
        graph::graphobject::{ObjectType, WithObjectType},
        sound::soundnumbersource::SoundNumberSourceId,
    },
    objects::functions::Constant,
    ui_core::{
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::OuterSoundNumberInputContext,
        summon_widget::{SummonWidgetState, SummonWidgetStateBuilder},
        ui_factory::UiFactory,
    },
};

#[derive(Copy, Clone)]
pub(super) enum NumberSummonValue {
    NumberSourceType(ObjectType),
    SoundNumberSource(SoundNumberSourceId),
}

pub(super) fn build_summon_widget_for_sound_number_input(
    position: egui::Pos2,
    ui_factory: &UiFactory<NumberGraphUi>,
    ctx: &OuterSoundNumberInputContext,
) -> SummonWidgetState<NumberSummonValue> {
    let mut builder = SummonWidgetStateBuilder::new(position);
    for object_type in ui_factory.all_object_types() {
        builder.add_basic_name(
            object_type.name().to_string(),
            NumberSummonValue::NumberSourceType(object_type),
        );
    }

    for snsid in ctx
        .temporal_layout()
        .available_number_sources(ctx.parent_sound_processor_id())
    {
        builder.add_basic_name(
            ctx.sound_graph_names().combined_number_source_name(*snsid),
            NumberSummonValue::SoundNumberSource(*snsid),
        );
    }

    // TODO: move this to the object ui after testing
    builder.add_pattern("constant".to_string(), |s| {
        // TODO: actually use the parsed value as part of initializing the constant
        // This should probably be done with a per-object/ui initialization type
        s.parse::<f32>()
            .ok()
            .and(Some(NumberSummonValue::NumberSourceType(Constant::TYPE)))
    });
    builder.build()
}
