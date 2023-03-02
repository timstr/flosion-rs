use crate::{
    core::{graphobject::ObjectId, soundprocessor::StaticSoundProcessorHandle},
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type HandleType = StaticSoundProcessorHandle<Dac>;
    type StateType = NoUIState;
    fn ui(
        &self,
        id: ObjectId,
        dac: StaticSoundProcessorHandle<Dac>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        ObjectWindow::new_sound_processor(id.as_sound_processor_id().unwrap())
            .add_left_peg(dac.input.id(), "Input")
            .show(ui.ctx(), graph_tools, |ui, _graph_tools| {
                ui.label("Dac");
                if ui.button("Reset").clicked() {
                    dac.reset();
                }
            });
    }
}
