use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::readwritewaveform::ReadWriteWaveform,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ReadWriteWaveformUi {}

impl SoundObjectUi for ReadWriteWaveformUi {
    type HandleType = DynamicSoundProcessorHandle<ReadWriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        rww: DynamicSoundProcessorHandle<ReadWriteWaveform>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&rww, "ReadWriteWaveform")
            .add_sound_input(rww.get().sound_input.id(), "input", sound_graph)
            .add_argument(rww.get().input_l.id(), "l")
            .add_argument(rww.get().input_r.id(), "r")
            .add_expression(rww.get().waveform.id(), "waveform", PlotConfig::new())
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["readwritewaveform"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
