use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow,
            SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type HandleType = DynamicSoundProcessorHandle<WaveGenerator>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        wavgen: DynamicSoundProcessorHandle<WaveGenerator>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("WaveGenerator");
            ui.add(NumberInputWidget::new(
                &wavgen.amplitude,
                "Amplitude",
                graph_tools,
            ));
            ui.add(NumberInputWidget::new(
                &wavgen.frequency,
                "Frequency",
                graph_tools,
            ));
            ui.add(NumberOutputWidget::new(&wavgen.time, "Time", graph_tools));
            ui.add(NumberOutputWidget::new(&wavgen.phase, "Phase", graph_tools));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
        });
    }
}
