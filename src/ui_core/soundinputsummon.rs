use eframe::egui;

use crate::core::{graph::graphobject::ObjectType, sound::soundprocessor::SoundProcessorId};

use super::{
    soundgraphui::SoundGraphUi,
    summon_widget::{SummonWidgetState, SummonWidgetStateBuilder},
    ui_factory::UiFactory,
};

#[derive(Copy, Clone)]
pub(super) enum SoundInputSummonValue {
    NewSoundProcessor(ObjectType),
    ExistingSoundProcessor(SoundProcessorId),
}

pub(super) fn build_summon_widget_for_sound_input(
    position: egui::Pos2,
    ui_factory: &UiFactory<SoundGraphUi>,
) -> SummonWidgetState<SoundInputSummonValue> {
    let mut builder = SummonWidgetStateBuilder::new(position);

    for object_type in ui_factory.all_object_types() {
        builder.add_basic_name(
            object_type.name().to_string(),
            SoundInputSummonValue::NewSoundProcessor(object_type),
        );
    }

    // TODO: add existing sound processors that would be legal to connect to

    builder.build()
}
