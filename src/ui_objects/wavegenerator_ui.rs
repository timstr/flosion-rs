use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedDynamicSoundProcessor},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow, SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type WrapperType = WrappedDynamicSoundProcessor<WaveGenerator>;
    type StateType = ();

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedDynamicSoundProcessor<WaveGenerator>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &(),
    ) {
        let id = id.as_sound_processor_id().unwrap();
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("WaveGenerator");
            ui.add(NumberInputWidget::new(
                object.amplitude.id(),
                "Amplitude",
                graph_tools,
            ));
            ui.add(NumberOutputWidget::new(
                object.time.id(),
                "Time",
                graph_tools,
            ));
            ui.add(NumberInputWidget::new(
                object.frequency.id(),
                "Frequency",
                graph_tools,
            ));
            ui.add(NumberOutputWidget::new(
                object.phase.id(),
                "Phase",
                graph_tools,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
        });
    }
}
