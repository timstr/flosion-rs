use rand::prelude::*;

use crate::{
    core::{
        graphobject::ObjectId, samplefrequency::SAMPLE_FREQUENCY,
        soundprocessor::DynamicSoundProcessorHandle,
    },
    objects::melody::Melody,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberOutputWidget, ObjectUi, ObjectWindow, SoundInputWidget,
            SoundOutputWidget,
        },
    },
};

#[derive(Default)]
pub struct MelodyUi {}

impl ObjectUi for MelodyUi {
    type HandleType = DynamicSoundProcessorHandle<Melody>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        melody: DynamicSoundProcessorHandle<Melody>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Melody");
            ui.add(SoundInputWidget::new(
                melody.input.id(),
                "Input",
                graph_tools,
            ));
            ui.add(NumberOutputWidget::new(
                melody.note_frequency.id(),
                "Note Frequency",
                graph_tools,
            ));
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));

            if ui.button("Randomize").clicked() {
                melody.clear();
                for _ in 0..16 {
                    let start_time_samples = thread_rng().gen::<usize>() % (4 * SAMPLE_FREQUENCY);
                    let duration_samples = thread_rng().gen::<usize>() % SAMPLE_FREQUENCY;
                    let frequency = 250.0 * thread_rng().gen::<f32>();
                    melody.add_note(start_time_samples, duration_samples, frequency);
                }
            }
        });
    }
}
