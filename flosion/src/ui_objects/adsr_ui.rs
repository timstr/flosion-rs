use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::adsr::ADSR,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct ADSRUi {}

impl SoundObjectUi for ADSRUi {
    type ObjectType = SoundProcessorWithId<ADSR>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        adsr: &mut SoundProcessorWithId<ADSR>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new(adsr.id(), "ADSR")
            .add_sound_input(&adsr.input, "input")
            .add_expression(&adsr.attack_time, "attack_time", PlotConfig::new())
            .add_expression(&adsr.decay_time, "decay_time", PlotConfig::new())
            .add_expression(&adsr.sustain_level, "sustain_level", PlotConfig::new())
            .add_expression(&adsr.release_time, "release_time", PlotConfig::new())
            .show(adsr, ui, ctx, graph_ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["adsr"]
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
