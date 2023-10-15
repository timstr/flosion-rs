use eframe::egui;

use crate::{
    core::sound::soundprocessor::StaticSoundProcessorHandle,
    objects::dac::Dac,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Dac>;
    type StateType = ();
    fn ui(
        &self,
        dac: StaticSoundProcessorHandle<Dac>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        ProcessorUi::new(dac.id(), "Dac", data.color)
            .add_sound_input(dac.input.id(), "input")
            .show_with(ui, ctx, ui_state, |ui, _ui_state| {
                if ui.add(egui::Button::new("Reset").wrap(false)).clicked() {
                    dac.reset();
                }
            });
    }
}
