use eframe::egui;

use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::audioclip::AudioClip,
    ui_core::{
        graph_ui_state::GraphUiState,
        object_ui::{ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct AudioClipUi {}

impl ObjectUi for AudioClipUi {
    type HandleType = DynamicSoundProcessorHandle<AudioClip>;
    type StateType = ();
    fn ui(
        &self,
        audioclip: DynamicSoundProcessorHandle<AudioClip>,
        ui_state: &mut GraphUiState,
        ui: &mut egui::Ui,
        data: ObjectUiData<()>,
    ) {
        ObjectWindow::new_sound_processor(audioclip.id(), "AudioClip", data.color)
            // .add_right_peg(audioclip.id(), "Output")
            .show(ui.ctx(), ui_state)
    }
}
