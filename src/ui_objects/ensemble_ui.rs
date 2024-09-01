use eframe::egui;

use crate::{
    core::{
        graph::graphobject::ObjectInitialization,
        sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    },
    objects::ensemble::Ensemble,
    ui_core::{
        expressionplot::PlotConfig, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
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
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&ensemble, "Ensemble")
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
                graph_ui_state,
                sound_graph,
                |ui, _ui_state, sound_graph| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(
                            egui::RichText::new("Voices")
                                .color(egui::Color32::from_black_alpha(192))
                                .italics(),
                        ));
                        // TODO: this currently triggers a full graph validation
                        // during every single UI redraw, even if nothing changed.
                        // Make this more efficient.
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

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: ObjectInitialization,
    ) -> Result<(), ()> {
        Ok(())
    }
}
