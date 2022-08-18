use crate::{
    core::{graphobject::ObjectId, soundprocessor::SoundProcessorHandle},
    objects::dac::Dac,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow, SoundInputWidget},
    },
};

#[derive(Default)]
pub struct DacUi {}

impl ObjectUi for DacUi {
    type WrapperType = SoundProcessorHandle<Dac>;
    type StateType = NoUIState;
    fn ui(
        &self,
        id: ObjectId,
        wrapper: &SoundProcessorHandle<Dac>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let object = wrapper.instance();
        ObjectWindow::new_sound_processor(id.as_sound_processor_id().unwrap()).show(
            ui.ctx(),
            graph_tools,
            |ui, graph_tools| {
                ui.label("Dac");
                // ui.separator();
                ui.add(SoundInputWidget::new(
                    object.input.id(),
                    "Output",
                    graph_tools,
                ));
                if ui.button("Reset").clicked() {
                    wrapper.instance().reset();
                }
            },
        );
    }
}
