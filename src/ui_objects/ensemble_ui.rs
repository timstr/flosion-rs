use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::ensemble::Ensemble,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow,
            SoundInputWidget, SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct EnsembleUi {}

impl ObjectUi for EnsembleUi {
    type HandleType = DynamicSoundProcessorHandle<Ensemble>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        ensemble: DynamicSoundProcessorHandle<Ensemble>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Ensemble");
            ui.add(SoundInputWidget::new(
                ensemble.input.id(),
                "Input",
                graph_tools,
            ));
            ui.add(NumberInputWidget::new(
                &ensemble.frequency_in,
                "Frequency In",
                graph_tools,
            ));
            ui.add(NumberInputWidget::new(
                &ensemble.frequency_spread,
                "Frequency Spread",
                graph_tools,
            ));
            ui.add(NumberOutputWidget::new(
                &ensemble.voice_frequency,
                "Voice Frequency",
                graph_tools,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
        });
    }
}
