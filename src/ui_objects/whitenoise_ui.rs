use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::whitenoise::WhiteNoise,
    ui_core::{
        arguments::ParsedArguments, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct WhiteNoiseUi {}

impl ObjectUi for WhiteNoiseUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<WhiteNoise>;
    type StateType = ();

    fn ui(
        &self,
        whitenoise: DynamicSoundProcessorHandle<WhiteNoise>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&whitenoise, "WhiteNoise").show(ui, ctx, graph_ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["whitenoise"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
