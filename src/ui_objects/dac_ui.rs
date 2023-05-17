use eframe::egui;

use crate::{
    core::soundprocessor::StaticSoundProcessorHandle,
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ProcessorUi},
        ui_context::UiContext,
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type HandleType = StaticSoundProcessorHandle<Dac>;
    type StateType = NoUIState;
    fn ui(
        &self,
        dac: StaticSoundProcessorHandle<Dac>,
        graph_tools: &mut GraphUIState,
        ui: &mut egui::Ui,
        ctx: &UiContext,
        data: ObjectUiData<NoUIState>,
    ) {
        ProcessorUi::new(dac.id(), "Dac", data.color)
            // .add_left_peg(dac.input.id(), "Input")
            .add_synchronous_sound_input(dac.input.id())
            .show_with(ui, ctx, graph_tools, |ui, _graph_tools| {
                if ui.add(egui::Button::new("Reset").wrap(false)).clicked() {
                    dac.reset();
                }
            });
    }
}
