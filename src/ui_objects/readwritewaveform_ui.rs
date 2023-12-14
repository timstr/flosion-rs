use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::readwritewaveform::ReadWriteWaveform,
    ui_core::{
        numberinputplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ReadWriteWaveformUi {}

impl ObjectUi for ReadWriteWaveformUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<ReadWriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        rww: DynamicSoundProcessorHandle<ReadWriteWaveform>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&rww, "ReadWriteWaveform", data.color)
            .add_sound_input(rww.sound_input.id(), "input", sound_graph)
            .add_number_source(rww.input_l.id(), "l")
            .add_number_source(rww.input_r.id(), "r")
            .add_number_input(rww.waveform.id(), "waveform", PlotConfig::new())
            .show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["readwritewaveform"]
    }
}
