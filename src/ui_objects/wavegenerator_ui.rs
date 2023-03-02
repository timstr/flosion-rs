use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
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
        ObjectWindow::new_sound_processor(id)
            .add_left_peg(&wavgen.amplitude, "Amplitude")
            .add_left_peg(&wavgen.frequency, "Frequency")
            .add_top_peg(&wavgen.time, "Time")
            .add_top_peg(&wavgen.phase, "Phase")
            .add_right_peg(wavgen.id(), "Output")
            .show(ui.ctx(), graph_tools, |ui, _graph_tools| {
                ui.label("WaveGenerator");
            });
    }
}
