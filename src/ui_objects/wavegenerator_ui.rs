use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ProcessorUi},
        ui_context::UiContext,
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type HandleType = DynamicSoundProcessorHandle<WaveGenerator>;
    type StateType = NoUIState;

    fn ui(
        &self,
        wavgen: DynamicSoundProcessorHandle<WaveGenerator>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        ctx: &UiContext,
        data: ObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(wavgen.id(), "WaveGenerator", data.color)
            // .add_top_peg(&wavgen.time, "Time")
            // .add_top_peg(&wavgen.phase, "Phase")
            // .add_right_peg(wavgen.id(), "Output")
            .add_number_input(wavgen.amplitude.id(), "Amplitude")
            .add_number_input(wavgen.frequency.id(), "Frequency")
            .show(ui, ctx, graph_tools);
    }
}
