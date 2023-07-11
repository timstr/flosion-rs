use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::wavegenerator::WaveGenerator,
    ui_core::{
        object_ui::{NoUIState, ObjectUi},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUIState,
        soundobjectuistate::ConcreteSoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WaveGeneratorUi {}

impl ObjectUi for WaveGeneratorUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WaveGenerator>;
    type StateType = NoUIState;

    fn ui(
        &self,
        wavgen: DynamicSoundProcessorHandle<WaveGenerator>,
        graph_tools: &mut SoundGraphUIState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        data: ConcreteSoundObjectUiData<NoUIState>,
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
