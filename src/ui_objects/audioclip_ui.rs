use eframe::egui;

use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::audioclip::AudioClip,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct AudioClipUi {}

impl ObjectUi for AudioClipUi {
    type HandleType = DynamicSoundProcessorHandle<AudioClip>;
    type StateType = NoUIState;
    fn ui(
        &self,
        audioclip: DynamicSoundProcessorHandle<AudioClip>,
        graph_tools: &mut GraphUIState,
        ui: &mut egui::Ui,
        data: ObjectUiData<NoUIState>,
    ) {
        ObjectWindow::new_sound_processor(audioclip.id(), "AudioClip", data.color)
            .add_right_peg(audioclip.id(), "Output")
            .show(ui.ctx(), graph_tools)
    }
}
