use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedDynamicSoundProcessor},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{
            NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow, SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type WrapperType = WrappedDynamicSoundProcessor<WaveGenerator>;

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedDynamicSoundProcessor<WaveGenerator>,
        graph_state: &mut GraphUITools,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("WaveGenerator");
            ui.add(NumberInputWidget::new(object.amplitude.id(), graph_state));
            ui.add(NumberInputWidget::new(object.frequency.id(), graph_state));
            ui.add(NumberOutputWidget::new(object.phase.id(), graph_state));
            ui.add(SoundOutputWidget::new(id, graph_state));
        });
    }
}
