use eframe::egui;

use crate::{
    core::{graphobject::ObjectId, soundprocessor::SoundProcessorHandle},
    objects::adsr::ADSR,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberInputWidget, ObjectUi, ObjectWindow, SoundInputWidget,
            SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct ADSRUi;

impl ObjectUi for ADSRUi {
    type WrapperType = SoundProcessorHandle<ADSR>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &SoundProcessorHandle<ADSR>,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_state, |ui, graph_state| {
            ui.label("ADSR");
            ui.add(SoundInputWidget::new(
                object.input.id(),
                "Input",
                graph_state,
            ));
            ui.add(NumberInputWidget::new(
                object.attack_time.id(),
                "Attack Time",
                graph_state,
            ));
            ui.add(NumberInputWidget::new(
                object.decay_time.id(),
                "Decay Time",
                graph_state,
            ));
            ui.add(NumberInputWidget::new(
                object.sustain_level.id(),
                "Sustain Level",
                graph_state,
            ));
            ui.add(NumberInputWidget::new(
                object.release_time.id(),
                "Release Time",
                graph_state,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_state));
        });
    }
}
