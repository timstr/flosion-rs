use eframe::egui;

use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::ensemble::Ensemble,
    ui_core::{
        arguments::ParsedArguments, expressionplot::PlotConfig, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct EnsembleUi {}

impl SoundObjectUi for EnsembleUi {
    type ObjectType = SoundProcessorWithId<Ensemble>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        ensemble: &mut SoundProcessorWithId<Ensemble>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        ProcessorUi::new("Ensemble")
            .add_sound_input(&ensemble.input, "input")
            .add_expression(&ensemble.frequency_in, &["frequency_in"], PlotConfig::new())
            .add_expression(
                &ensemble.frequency_spread,
                &["frequency_spread"],
                PlotConfig::new(),
            )
            .add_argument(&ensemble.voice_frequency, "voice_frequency")
            .show_with(
                ensemble,
                ui,
                ctx,
                graph_ui_state,
                |ensemble, ui, _ui_state| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new("Voices")
                                .color(egui::Color32::from_black_alpha(192))
                                .italics(),
                        ));

                        let mut num_voices = ensemble.num_voices();
                        let r = ui.add(egui::Slider::new(&mut num_voices, 0..=16));
                        if r.changed() {
                            ensemble.set_num_voices(num_voices);
                        }
                    });
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["ensemble"]
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
