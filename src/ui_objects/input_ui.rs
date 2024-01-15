use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
    objects::input::Input,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct InputUi {}

impl ObjectUi for InputUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Input>;
    type StateType = ();
    fn ui(
        &self,
        input: StaticSoundProcessorHandle<Input>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
        sound_graph: &mut SoundGraph,
    ) {
        // TODO: controls for choosing input device?
        // Would require changes to input
        ProcessorUi::new(&input, "Input", data.color).show(ui, ctx, ui_state, sound_graph);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["input"]
    }
}
