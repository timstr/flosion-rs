use crate::{
    core::{graphobject::ObjectId, soundprocessor::StaticSoundProcessorHandle},
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow, SoundInputWidget},
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
        ObjectWindow::new_sound_processor(id.as_sound_processor_id().unwrap()).show(
            ui.ctx(),
            graph_tools,
            |ui, graph_tools| {
                ui.label("Dac");
                // ui.separator();
                ui.add(SoundInputWidget::new(dac.input.id(), "Output", graph_tools));
                if ui.button("Reset").clicked() {
                    dac.reset();
                }
            },
        );
    }
}
