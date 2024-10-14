use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::writewaveform::WriteWaveform,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WriteWaveformUi {}

impl SoundObjectUi for WriteWaveformUi {
    type HandleType = DynamicSoundProcessorHandle<WriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        ww: DynamicSoundProcessorHandle<WriteWaveform>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&ww, "WriteWaveform")
            .add_expression(ww.get().waveform.id(), "waveform", PlotConfig::new())
            .show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["writewaveform"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
