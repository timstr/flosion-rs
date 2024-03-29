use eframe::egui;

use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::adsr::ADSR,
    ui_core::{
        graph_ui_state::GraphUiState,
        object_ui::{ObjectUi, ObjectUiData, ObjectWindow},
    },
};

#[derive(Default)]
pub struct ADSRUi;

impl ObjectUi for ADSRUi {
    type HandleType = DynamicSoundProcessorHandle<ADSR>;
    type StateType = ();

    fn ui(
        &self,
        adsr: DynamicSoundProcessorHandle<ADSR>,
        graph_state: &mut GraphUiState,
        ui: &mut egui::Ui,
        data: ObjectUiData<()>,
    ) {
        ObjectWindow::new_sound_processor(adsr.id(), "ADSR", data.color)
            // .add_top_peg(&adsr.attack_time, "Attack Time")
            // .add_top_peg(&adsr.decay_time, "Decay Time")
            // .add_top_peg(&adsr.sustain_level, "Sustain Level")
            // .add_top_peg(&adsr.release_time, "Release Time")
            // .add_left_peg(adsr.input.id(), "Input")
            // .add_right_peg(adsr.id(), "Output")
            .show(ui.ctx(), graph_state);
    }
}
