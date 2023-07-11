use eframe::egui;

use crate::{
    core::sound::soundprocessor::StaticSoundProcessorHandle,
    objects::dac::Dac,
    ui_core::{
        object_ui::{NoUIState, ObjectUi},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUIState,
        soundobjectuistate::ConcreteSoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Dac>;
    type StateType = NoUIState;
    fn ui(
        &self,
        dac: StaticSoundProcessorHandle<Dac>,
        graph_tools: &mut SoundGraphUIState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        data: ConcreteSoundObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(dac.id(), "Dac", data.color)
            // .add_left_peg(dac.input.id(), "Input")
            .add_sound_input(dac.input.id())
            .show_with(ui, ctx, graph_tools, |ui, _graph_tools| {
                if ui.add(egui::Button::new("Reset").wrap(false)).clicked() {
                    dac.reset();
                }
            });
    }
}
