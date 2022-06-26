use eframe::egui;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedDynamicSoundProcessor},
    objects::audioclip::AudioClip,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{ObjectUi, ObjectWindow, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct AudioClipUi {}

impl ObjectUi for AudioClipUi {
    type WrapperType = WrappedDynamicSoundProcessor<AudioClip>;
    type StateType = ();
    fn ui(
        &self,
        id: ObjectId,
        _object: &WrappedDynamicSoundProcessor<AudioClip>,
        graph_tools: &mut GraphUIState,
        ui: &mut egui::Ui,
        _state: &(),
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("AudioClip");
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
        })
    }
}
