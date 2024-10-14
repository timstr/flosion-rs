use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
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
    type ObjectType = SoundProcessorWithId<WriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        ww: &mut SoundProcessorWithId<WriteWaveform>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
    ) {
        ProcessorUi::new(ww.id(), "WriteWaveform")
            .add_expression(&ww.waveform, "waveform", PlotConfig::new())
            .show(ww, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["writewaveform"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::ObjectType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
