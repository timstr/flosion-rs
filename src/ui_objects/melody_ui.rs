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
    bpm: f32,
    divisions_per_beat: u8,
}

impl Default for MelodyUiState {
    fn default() -> Self {
        MelodyUiState {
            width: 400.0,
            height: 200.0,
            bpm: 120.0,
            divisions_per_beat: 4,
        }
    }
}

impl Serializable for MelodyUiState {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.f32(self.width);
        serializer.f32(self.height);
        serializer.f32(self.bpm);
        serializer.u8(self.divisions_per_beat);
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(MelodyUiState {
            width: deserializer.f32()?,
            height: deserializer.f32()?,
            bpm: deserializer.f32()?,
            divisions_per_beat: deserializer.u8()?,
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

        let bps = state.bpm / 60.0;
        let dps = bps * (state.divisions_per_beat as f32);

        let num_divs = (melody_duration_seconds * dps).ceil() as usize;

        for i in 1..num_divs {
            let t = (i as f32) / dps;
            let x = (t / melody_duration_seconds) * state.width;
            let alpha = if (i % state.divisions_per_beat as usize) == 0 {
                192
            } else {
                64
            };
            let stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(alpha));
            painter.add(egui::Shape::line_segment(
                [
                    egui::pos2(response.rect.left() + x, response.rect.top()),
                    egui::pos2(response.rect.left() + x, response.rect.bottom()),
                ],
                stroke,
            ));
        }

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

                let cursor_offset_id = egui::Id::new("note_drag_cursor_offset");

                if note_response.drag_started() {
                    let offset: egui::Vec2 =
                        ui.ctx().pointer_interact_pos().unwrap() - note_ui_rect.left_top();
                    ui.memory_mut(|m| m.data.insert_temp(cursor_offset_id, offset));
                }

                if note_response.dragged() {
                    let cursor_offset =
                        ui.memory_mut(|m| m.data.get_temp::<egui::Vec2>(cursor_offset_id).unwrap());

                    // let deltas = note_response.drag_delta();
                    let cursor_pos = ui.ctx().pointer_interact_pos().unwrap() - cursor_offset;

                    // workaround because Rect::transform_vec is not a thing yet
                    let time_and_freq = screen_to_time_frequency.transform_pos(cursor_pos);

                    let new_time = time_and_freq.x;
                    let new_freq = max_freq - time_and_freq.y;

                    let freq_snap = 25.0;
                    let new_freq = (new_freq / freq_snap).round() * freq_snap;

                    melody.edit_note(
                        *note_id,
                        Note {
                            start_time_samples: ((new_time) * SAMPLE_FREQUENCY as f32).max(0.0)
                                as usize,
                            duration_samples: note.duration_samples,
                            frequency: new_freq,
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
                ui.horizontal(|ui| {
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

                    ui.separator();

                    ui.label("BPM");

                    let prev_bps = data.state.bpm / 60.0;
                    let r = ui.add(egui::Slider::new(&mut data.state.bpm, 1.0..=240.0));
                    if r.changed() {
                        let new_bps = data.state.bpm / 60.0;
                        let mut notes = melody.notes();
                        for (_note_id, note) in &mut notes {
                            let start_time =
                                (note.start_time_samples as f32) / (SAMPLE_FREQUENCY as f32);
                            let start_beats = start_time * prev_bps;
                            let new_start_time = start_beats / new_bps;
                            note.start_time_samples =
                                (new_start_time * SAMPLE_FREQUENCY as f32) as usize;
                        }
                        melody.set_notes(notes);
                    }

                    ui.separator();

                    ui.label("Divisions");

                    ui.add(
                        egui::DragValue::new(&mut data.state.divisions_per_beat)
                            .clamp_range(1..=32),
                    );
                });

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
