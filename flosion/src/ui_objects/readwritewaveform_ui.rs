use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::readwritewaveform::ReadWriteWaveform,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ReadWriteWaveformUi {}

impl SoundObjectUi for ReadWriteWaveformUi {
    type ObjectType = SoundProcessorWithId<ReadWriteWaveform>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        rww: &mut SoundProcessorWithId<ReadWriteWaveform>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new("ReadWriteWaveform")
            .add_sound_input(&rww.sound_input, "input")
            .add_argument(&rww.input_l, "l")
            .add_argument(&rww.input_r, "r")
            .add_expression(&rww.waveform, &["l", "r"], PlotConfig::new())
            .show(rww, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["readwritewaveform"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<NoObjectUiState, ()> {
        Ok(NoObjectUiState)
    }
}
