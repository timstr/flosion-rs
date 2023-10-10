use crate::core::{
    number::{numbergraphtopology::NumberGraphTopology, numbersource::NumberSourceId},
    sound::{soundnumberinput::SoundNumberInputId, soundprocessor::SoundProcessorId},
};

use super::{
    graph_ui::GraphUiContext,
    numbergraphui::NumberGraphUi,
    numbergraphuistate::{AnyNumberObjectUiData, NumberObjectUiStates},
    temporallayout::TemporalLayout,
    ui_factory::UiFactory,
};

#[derive(Clone, Copy)]
pub(crate) struct OuterSoundNumberInputContext<'a> {
    sound_number_input_id: SoundNumberInputId,
    parent_sound_processor_id: SoundProcessorId,
    temporal_layout: &'a TemporalLayout,
}

impl<'a> OuterSoundNumberInputContext<'a> {
    pub(super) fn new(
        sound_number_input_id: SoundNumberInputId,
        parent_sound_processor_id: SoundProcessorId,
        temporal_layout: &'a TemporalLayout,
    ) -> Self {
        Self {
            sound_number_input_id,
            parent_sound_processor_id,
            temporal_layout,
        }
    }

    pub(super) fn sound_number_input_id(&self) -> SoundNumberInputId {
        self.sound_number_input_id
    }

    pub(super) fn parent_sound_processor_id(&self) -> SoundProcessorId {
        self.parent_sound_processor_id
    }

    pub(super) fn temporal_layout(&self) -> &TemporalLayout {
        self.temporal_layout
    }
}

#[derive(Clone, Copy)]
pub(crate) enum OuterNumberGraphUiContext<'a> {
    // TODO: top level number graph/function also
    SoundNumberInput(OuterSoundNumberInputContext<'a>),
}

impl<'a> From<OuterSoundNumberInputContext<'a>> for OuterNumberGraphUiContext<'a> {
    fn from(value: OuterSoundNumberInputContext<'a>) -> Self {
        OuterNumberGraphUiContext::SoundNumberInput(value)
    }
}

pub struct NumberGraphUiContext<'a> {
    ui_factory: &'a UiFactory<NumberGraphUi>,
    object_states: &'a NumberObjectUiStates,
    topology: &'a NumberGraphTopology,
}

impl<'a> NumberGraphUiContext<'a> {
    pub(super) fn new(
        ui_factory: &'a UiFactory<NumberGraphUi>,
        object_states: &'a NumberObjectUiStates,
        topology: &'a NumberGraphTopology,
    ) -> NumberGraphUiContext<'a> {
        NumberGraphUiContext {
            ui_factory,
            object_states,
            topology,
        }
    }

    pub(super) fn ui_factory(&self) -> &UiFactory<NumberGraphUi> {
        self.ui_factory
    }

    pub(super) fn object_ui_states(&self) -> &NumberObjectUiStates {
        self.object_states
    }

    pub(super) fn topology(&self) -> &NumberGraphTopology {
        self.topology
    }
}

impl<'a> GraphUiContext<'a> for NumberGraphUiContext<'a> {
    type GraphUi = NumberGraphUi;

    fn get_object_ui_data(&self, id: NumberSourceId) -> &AnyNumberObjectUiData {
        todo!()
    }
}
