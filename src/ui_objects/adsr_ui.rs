use eframe::egui;

use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::adsr::ADSR,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct ADSRUi;

impl ObjectUi for ADSRUi {
    type HandleType = DynamicSoundProcessorHandle<ADSR>;
    type StateType = NoUIState;

    fn ui(
        &self,
        adsr: DynamicSoundProcessorHandle<ADSR>,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
        _state: &NoUIState,
    ) {
        ObjectWindow::new_sound_processor(adsr.id())
            .add_top_peg(&adsr.attack_time, "Attack Time")
            .add_top_peg(&adsr.decay_time, "Decay Time")
            .add_top_peg(&adsr.sustain_level, "Sustain Level")
            .add_top_peg(&adsr.release_time, "Release Time")
            .add_left_peg(adsr.input.id(), "Input")
            .add_right_peg(adsr.id(), "Output")
            .show(ui.ctx(), graph_state, |ui, _graph_state| {
                ui.label("ADSR");
            });
    }
}
