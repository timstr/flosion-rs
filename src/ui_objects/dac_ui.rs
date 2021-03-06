use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedStaticSoundProcessor},
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{ObjectUi, ObjectWindow, SoundInputWidget},
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type WrapperType = WrappedStaticSoundProcessor<Dac>;
    type StateType = ();
    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedStaticSoundProcessor<Dac>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &(),
    ) {
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id.as_sound_processor_id().unwrap()).show(
            ui.ctx(),
            graph_tools,
            |ui, graph_tools| {
                ui.label("Dac");
                // ui.separator();
                ui.label(if object.is_playing() {
                    "Playing"
                } else {
                    "Paused"
                });
                ui.add(SoundInputWidget::new(
                    object.input().id(),
                    "Output",
                    graph_tools,
                ));
                if ui.button("Reset").clicked() {
                    wrapper.instance().reset();
                }
            },
        );
    }
}
