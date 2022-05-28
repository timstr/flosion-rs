use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedStaticSoundProcessor},
    objects::dac::Dac,
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{ObjectUi, ObjectWindow, SoundInputWidget},
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type WrapperType = WrappedStaticSoundProcessor<Dac>;
    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedStaticSoundProcessor<Dac>,
        graph_tools: &mut GraphUITools,
        ui: &mut eframe::egui::Ui,
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
                    todo!();
                }
            },
        );
    }
}
