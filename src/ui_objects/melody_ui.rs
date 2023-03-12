use eframe::{egui, emath};

use rand::prelude::*;

use crate::{
    core::{
        samplefrequency::SAMPLE_FREQUENCY,
        serialization::{Deserializer, Serializable, Serializer},
        soundprocessor::DynamicSoundProcessorHandle,
        uniqueid::UniqueId,
    },
    objects::melody::{Melody, Note},
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{ObjectUi, ObjectUiData, ObjectWindow},
    },
};

pub struct MelodyUiState {
    width: f32,
    height: f32,
}

impl Default for MelodyUiState {
    fn default() -> Self {
        MelodyUiState {
            width: 400.0,
            height: 200.0,
        }
    }
}

impl Serializable for MelodyUiState {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.f32(self.width);
        serializer.f32(self.height);
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(MelodyUiState {
            width: deserializer.f32()?,
            height: deserializer.f32()?,
        })
    }
}

#[derive(Default)]
pub struct MelodyUi {}

impl MelodyUi {
    fn ui_content(
        &self,
        ui: &mut egui::Ui,
        melody: &DynamicSoundProcessorHandle<Melody>,
        state: &MelodyUiState,
    ) -> egui::Response {
        let (response, painter) = ui.allocate_painter(
            egui::Vec2::new(state.width, state.height),
            egui::Sense::hover(),
        );

        let melody_duration = melody.length_samples();
        let melody_duration_seconds = melody_duration as f32 / SAMPLE_FREQUENCY as f32;

        let max_freq = 500.0;
        let time_frequency_rect = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(melody_duration_seconds, max_freq),
        );

        let note_freq_height = max_freq * 0.025;

        let time_frequency_to_screen =
            emath::RectTransform::from_to(time_frequency_rect, response.rect);

        let screen_to_time_frequency = time_frequency_to_screen.inverse();

        let note_rects: Vec<egui::Rect> = melody
            .notes()
            .iter()
            .map(|(note_id, note)| {
                let note_start_seconds = note.start_time_samples as f32 / SAMPLE_FREQUENCY as f32;
                let note_duration_seconds = note.duration_samples as f32 / SAMPLE_FREQUENCY as f32;

                let note_tf_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        note_start_seconds,
                        max_freq - note.frequency - 0.5 * note_freq_height,
                    ),
                    egui::pos2(
                        note_start_seconds + note_duration_seconds,
                        max_freq - note.frequency + 0.5 * note_freq_height,
                    ),
                );

                let note_ui_rect = time_frequency_to_screen.transform_rect(note_tf_rect);

                let note_ui_id = response.id.with(note_id.value());
                let note_response =
                    ui.interact(note_ui_rect, note_ui_id, egui::Sense::click_and_drag());

                if note_response.dragged() {
                    let deltas = note_response.drag_delta();

                    // workaround because Rect::transform_vec is not a thing yet
                    let p0 = screen_to_time_frequency.transform_pos(egui::pos2(0.0, 0.0));
                    let p1 = screen_to_time_frequency.transform_pos(deltas.to_pos2());

                    let delta_time = p1.x - p0.x;
                    let delta_freq = p0.y - p1.y;

                    melody.edit_note(
                        *note_id,
                        Note {
                            start_time_samples: ((note_start_seconds + delta_time)
                                * SAMPLE_FREQUENCY as f32)
                                .max(0.0) as usize,
                            duration_samples: note.duration_samples,
                            frequency: note.frequency + delta_freq,
                        },
                    );
                }

                note_ui_rect
            })
            .collect();

        let note_shapes: Vec<egui::Shape> = note_rects
            .into_iter()
            .flat_map(|note_ui_rect| {
                [
                    egui::Shape::rect_filled(note_ui_rect, 0.0, egui::Color32::BLUE),
                    egui::Shape::rect_stroke(note_ui_rect, 0.0, (1.0, egui::Color32::WHITE)),
                ]
            })
            .collect();

        painter.add(note_shapes);

        response
    }
}

impl ObjectUi for MelodyUi {
    type HandleType = DynamicSoundProcessorHandle<Melody>;
    type StateType = MelodyUiState;

    fn ui(
        &self,
        melody: DynamicSoundProcessorHandle<Melody>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        data: ObjectUiData<MelodyUiState>,
    ) {
        ObjectWindow::new_sound_processor(melody.id(), "Melody", data.color)
            .add_left_peg(melody.input.id(), "Input")
            .add_left_peg(&melody.melody_time, "Melody Time")
            .add_left_peg(&melody.note_frequency, "Note Frequency")
            .add_left_peg(&melody.note_time, "Note Time")
            .add_left_peg(&melody.note_progress, "Note Progress")
            .add_right_peg(melody.id(), "Output")
            .show_with(ui.ctx(), graph_tools, |ui, _graph_tools| {
                if ui.button("Randomize").clicked() {
                    melody.clear();
                    for _ in 0..16 {
                        let start_time_samples =
                            thread_rng().gen::<usize>() % (4 * SAMPLE_FREQUENCY);
                        let duration_samples = thread_rng().gen::<usize>() % SAMPLE_FREQUENCY;
                        let frequency = 50.0 * (thread_rng().gen::<usize>() % 10) as f32;
                        let note = Note {
                            start_time_samples,
                            duration_samples,
                            frequency,
                        };
                        melody.add_note(note);
                    }
                }

                let canvas_response = egui::Frame::canvas(ui.style())
                    .show(ui, |ui| self.ui_content(ui, &melody, data.state));

                let canvas_bottom_right = canvas_response.response.rect.right_bottom();
                ui.put(
                    egui::Rect::from_min_max(
                        canvas_bottom_right,
                        canvas_bottom_right + egui::Vec2::splat(10.0),
                    ),
                    |ui: &mut egui::Ui| -> egui::Response {
                        let dragger_frame = egui::Frame::default()
                            .fill(egui::Color32::DARK_BLUE)
                            .stroke(egui::Stroke::new(1.0, egui::Color32::WHITE));

                        let r = dragger_frame.show(ui, |ui| {
                            ui.allocate_response(ui.available_size(), egui::Sense::drag())
                        });

                        if r.inner.dragged() {
                            let delta = r.inner.drag_delta();
                            data.state.width = (data.state.width + delta.x).max(0.0);
                            data.state.height = (data.state.height + delta.y).max(0.0);
                        }
                        r.response
                    },
                );
            });
    }
}
