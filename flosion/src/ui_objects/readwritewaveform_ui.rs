use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
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
    type ObjectType = SoundProcessorWithId<ReadWriteWaveform>;
    type StateType = ();

    fn ui(
        &self,
        rww: &mut SoundProcessorWithId<ReadWriteWaveform>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
    ) {
        ProcessorUi::new(rww.id(), "ReadWriteWaveform")
            .add_sound_input(rww.sound_input.id(), "input")
            .add_processor_argument(rww.input_l.id(), "l")
            .add_processor_argument(rww.input_r.id(), "r")
            .add_expression(&rww.waveform, "waveform", PlotConfig::new())
            .show(rww, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["readwritewaveform"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::ObjectType, _args: &ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
