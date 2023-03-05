use eframe::egui;

use crate::{
    core::soundprocessor::StaticSoundProcessorHandle,
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectUiData, ObjectWindow},
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
        ui: &mut eframe::egui::Ui,
        data: ObjectUiData<NoUIState>,
    ) {
        ObjectWindow::new_sound_processor(dac.id(), "Dac", data.color)
            .add_left_peg(dac.input.id(), "Input")
            .show_with(ui.ctx(), graph_tools, |ui, _graph_tools| {
                if ui.add(egui::Button::new("Reset").wrap(false)).clicked() {
                    dac.reset();
                }
            });
    }
}
