use eframe::egui;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::audioclip::AudioClip,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct AudioClipUi {}

impl ObjectUi for AudioClipUi {
    type HandleType = DynamicSoundProcessorHandle<AudioClip>;
    type StateType = NoUIState;
    fn ui(
        &self,
        id: ObjectId,
        _object: DynamicSoundProcessorHandle<AudioClip>,
        graph_tools: &mut GraphUIState,
        ui: &mut egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id)
            .add_right_peg(id, "Output")
            .show(ui.ctx(), graph_tools, |ui, _graph_tools| {
                ui.label("AudioClip");
            })
    }
}
