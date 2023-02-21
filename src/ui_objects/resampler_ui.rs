use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::resampler::Resampler,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberInputWidget, ObjectUi, ObjectWindow, SoundInputWidget,
            SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct ResamplerUi {}

impl ObjectUi for ResamplerUi {
    type HandleType = DynamicSoundProcessorHandle<Resampler>;
    type StateType = NoUIState;
    fn ui(
        &self,
        id: ObjectId,
        resampler: DynamicSoundProcessorHandle<Resampler>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Resampler");
            ui.add(SoundInputWidget::new(
                resampler.input.id(),
                "Input",
                graph_tools,
            ));
            ui.add(NumberInputWidget::new(
                &resampler.speed_ratio,
                "Speed Ratio",
                graph_tools,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
        });
    }
}
