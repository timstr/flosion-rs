use eframe::egui;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
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
        id: ObjectId,
        object: DynamicSoundProcessorHandle<ADSR>,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id)
            .add_top_peg(&object.attack_time, "Attack Time")
            .add_top_peg(&object.decay_time, "Decay Time")
            .add_top_peg(&object.sustain_level, "Sustain Level")
            .add_top_peg(&object.release_time, "Release Time")
            .add_left_peg(object.input.id(), "Input")
            .add_right_peg(object.id(), "Output")
            .show(ui.ctx(), graph_state, |ui, _graph_state| {
                ui.label("ADSR");
            });
    }
}
