use eframe::egui;

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

impl MelodyUi {
    fn ui_content(
        &self,
        ui: &mut egui::Ui,
        melody: &DynamicSoundProcessorHandle<Melody>,
    ) -> egui::Response {
        let (response, painter) =
            ui.allocate_painter(egui::Vec2::new(200.0, 300.0), egui::Sense::hover());

        let melody_duration = melody.length_samples();

        let note_shapes: Vec<egui::Shape> = melody
            .notes()
            .iter()
            .map(|note| {
                let x = (note.start_time_samples as f32 / melody_duration as f32)
                    * response.rect.width()
                    + response.rect.left();

                let w =
                    (note.duration_samples as f32 / melody_duration as f32) * response.rect.width();

                let y =
                    (1.0 - note.frequency / 500.0) * response.rect.height() + response.rect.top();

                let height = 5.0;

                let rect = egui::Rect::from_min_max(
                    egui::pos2(x, y - 0.5 * height),
                    egui::pos2(x + w, y + 0.5 * height),
                );

                egui::Shape::rect_stroke(rect, 0.0, (1.0, egui::Color32::WHITE))
            })
            .collect();

        painter.add(note_shapes);

        response
    }
}

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
                    let frequency = 25.0 * (thread_rng().gen::<usize>() % 20) as f32;
                    melody.add_note(start_time_samples, duration_samples, frequency);
                }
            }

            egui::Frame::canvas(ui.style()).show(ui, |ui| self.ui_content(ui, &melody));
        });
    }
}
