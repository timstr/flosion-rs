use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::ensemble::Ensemble,
    ui_core::{
        expressionplot::PlotConfig,
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct EnsembleUi {}

impl ObjectUi for EnsembleUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Ensemble>;
    type StateType = ();

    fn ui(
        &self,
        ensemble: DynamicSoundProcessorHandle<Ensemble>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&ensemble, "Ensemble", data.color)
            .add_sound_input(ensemble.input.id(), "input", sound_graph)
            .add_expression(
                ensemble.frequency_in.id(),
                "frequency_in",
                PlotConfig::new(),
            )
            .add_expression(
                ensemble.frequency_spread.id(),
                "frequency_spread",
                PlotConfig::new(),
            )
            .add_argument(ensemble.voice_frequency.id(), "voice_frequency")
            .show_with(
                ui,
                ctx,
                ui_state,
                sound_graph,
                |ui, _ui_state, sound_graph| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new("Voices")
                                .color(egui::Color32::from_black_alpha(192))
                                .italics(),
                        ));
                        let res = sound_graph.with_processor_tools(ensemble.id(), |mut tools| {
                            let mut num_voices = ensemble.num_voices(&tools);
                            let r = ui.add(egui::Slider::new(&mut num_voices, 0..=16));
                            if r.changed() {
                                ensemble.set_num_voices(num_voices, &mut tools);
                            }
                            Ok(())
                        });
                        if let Err(e) = res {
                            println!("Can't do that: {}", e.explain(sound_graph.topology()));
                        }
                    });
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["ensemble"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        ((), Color::default())
    }
}
