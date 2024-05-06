use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::{
        sound::{soundgraph::SoundGraph, soundprocessor::StaticSoundProcessorHandle},
        soundchunk::SoundChunk,
    },
    objects::oscilloscope::Oscilloscope,
    ui_core::{
        graph_ui::ObjectUiState,
        object_ui::{Color, ObjectUi, UiInitialization},
        soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState,
        soundobjectuistate::SoundObjectUiData,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct OscilloscopeUi {}

pub struct OscilloscopeUiState {
    buffer_reader: spmcq::Reader<SoundChunk>,
    exposure: f32,
    size: f32,
    gain: f32,
    decay: f32,
    rotation: u8,
    flip: bool,
    prev_sample: (f32, f32),
    image: ColorImage,
    texture: Option<TextureHandle>,
}

// TODO: this doesn't make sense
impl Serializable for OscilloscopeUiState {
    fn serialize(&self, serializer: &mut Serializer) {}

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Err(())
    }
}

impl ObjectUiState for OscilloscopeUiState {}

impl OscilloscopeUi {
    fn draw_line(
        mut x0: f32,
        mut y0: f32,
        mut x1: f32,
        mut y1: f32,
        image: &mut ColorImage,
        exposure: f32,
    ) {
        // Xiaolin Wu's line algorithm
        // https://en.wikipedia.org/wiki/Xiaolin_Wu%27s_line_algorithm

        let intensity = (exposure.max(0.0) / (x1 - x0).hypot(y1 - y0).max(1.0)).min(1.0);

        let mut plot = |x: isize, y: isize, c: f32| {
            if x < 0 || x >= image.width() as isize || y < 0 || y >= image.height() as isize {
                return;
            }
            let c = (c * intensity).clamp(0.0, 1.0);
            let idx = (y as usize * image.width()) + x as usize;
            let [r, g, b, _] = image.pixels[idx].to_array();
            image.pixels[idx] = Color32::from_rgba_premultiplied(
                r.saturating_add((c * 16.0).round() as u8),
                g.saturating_add((c * 255.0).round() as u8),
                b.saturating_add((c * 32.0).round() as u8),
                255,
            );
        };

        let steep = (y1 - y0).abs() > (x1 - x0).abs();

        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;

        let gradient = if dx.abs() < 1e-6 { 1.0 } else { dy / dx };

        // handle first endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x1 + 0.5).fract();
        let xpxl1 = xend as isize;
        let ypxl1 = yend.floor() as isize;

        if steep {
            plot(ypxl1, xpxl1, (1.0 - yend.fract()) * xgap);
            plot(ypxl1 + 1, xpxl1, yend.fract() * xgap);
        } else {
            plot(xpxl1, ypxl1, (1.0 - yend.fract()) * xgap);
            plot(xpxl1, ypxl1 + 1, yend.fract() * xgap);
        }
        let mut intery = yend + gradient;

        // handle second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as isize;
        let ypxl2 = yend.floor() as isize;

        if steep {
            plot(ypxl2, xpxl2, (1.0 - yend.fract()) * xgap);
            plot(ypxl2 + 1, xpxl2, yend.fract() * xgap);
        } else {
            plot(xpxl2, ypxl2, (1.0 - yend.fract()) * xgap);
            plot(xpxl2, ypxl2 + 1, yend.fract() * xgap);
        }

        // main loop
        if steep {
            for x in (xpxl1 + 1)..(xpxl2) {
                plot(intery.floor() as isize, x, 1.0 - intery.fract());
                plot(intery.floor() as isize + 1, x, intery.fract());
                intery += gradient;
            }
        } else {
            for x in (xpxl1 + 1)..xpxl2 {
                plot(x, intery.floor() as isize, 1.0 - intery.fract());
                plot(x, intery.floor() as isize, intery.fract());
                intery += gradient;
            }
        }
    }

    fn update_image(state: &mut OscilloscopeUiState) {
        while let Some(chunk) = state.buffer_reader.read().value() {
            let img = &mut state.image;
            let w = img.width() as f32;
            let h = img.height() as f32;

            for c in &mut img.pixels {
                let [r, g, b, _] = c.to_array();
                *c = Color32::from_rgba_premultiplied(
                    r.saturating_sub(((r as f32 * state.decay).round() as u8).max(1)),
                    g.saturating_sub(((g as f32 * state.decay).round() as u8).max(1)),
                    b.saturating_sub(((b as f32 * state.decay).round() as u8).max(1)),
                    255,
                );
            }

            let theta = -(state.rotation as f32) * std::f32::consts::FRAC_PI_4;

            let (sin_theta, cos_theta) = theta.sin_cos();

            let mut s_prev = state.prev_sample;
            for s in chunk.samples() {
                let s = if state.flip { (s.1, s.0) } else { s };
                let s = (
                    s.0 * cos_theta + s.1 * sin_theta,
                    s.0 * -sin_theta + s.1 * cos_theta,
                );

                let x0 = (0.5 + 0.5 * state.gain * s_prev.0).clamp(0.0, 1.0) * w;
                let y0 = (0.5 - 0.5 * state.gain * s_prev.1).clamp(0.0, 1.0) * h;
                let x1 = (0.5 + 0.5 * state.gain * s.0).clamp(0.0, 1.0) * w;
                let y1 = (0.5 - 0.5 * state.gain * s.1).clamp(0.0, 1.0) * h;

                Self::draw_line(x0, y0, x1, y1, img, state.exposure);
                s_prev = s;
            }
            state.prev_sample = s_prev;
        }
    }
}

impl ObjectUi for OscilloscopeUi {
    type GraphUi = SoundGraphUi;
    type HandleType = StaticSoundProcessorHandle<Oscilloscope>;
    type StateType = OscilloscopeUiState;

    fn ui<'a, 'b>(
        &self,
        oscilloscope: StaticSoundProcessorHandle<Oscilloscope>,
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        mut data: SoundObjectUiData<Self::StateType>,
        sound_graph: &mut SoundGraph,
    ) {
        ProcessorUi::new(&oscilloscope, "Oscilloscope", data.color)
            .add_sound_input(oscilloscope.input.id(), "Input", sound_graph)
            .show_with(
                ui,
                ctx,
                ui_state,
                sound_graph,
                |ui, _ui_state, _sound_graph| {
                    ui.vertical(|ui| {
                        Self::update_image(&mut data.state);

                        let texture_id = match data.state.texture.as_mut() {
                            Some(texture) => {
                                texture.set(data.state.image.clone(), TextureOptions::default());
                                texture.id()
                            }
                            None => {
                                let texture = ui.ctx().load_texture(
                                    "oscilloscope",
                                    data.state.image.clone(),
                                    TextureOptions::default(),
                                );
                                let id = texture.id();
                                data.state.texture = Some(texture);
                                id
                            }
                        };

                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Slider::new(&mut data.state.exposure, 0.0..=100.0)
                                    .logarithmic(true),
                            );
                            ui.separator();
                            ui.add(egui::Label::new(
                                egui::RichText::new("Beam Strength")
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Slider::new(&mut data.state.gain, 0.0..=100.0)
                                    .logarithmic(true),
                            );
                            ui.separator();
                            ui.add(egui::Label::new(
                                egui::RichText::new("Gain")
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Slider::new(&mut data.state.decay, 0.0..=1.0)
                                    .logarithmic(true),
                            );
                            ui.separator();
                            ui.add(egui::Label::new(
                                egui::RichText::new("Decay")
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut data.state.rotation, 0..=8));
                            ui.separator();
                            ui.add(egui::Label::new(
                                egui::RichText::new("Rotation")
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.add(egui::Checkbox::new(&mut data.state.flip, ""));
                            ui.separator();
                            ui.add(egui::Label::new(
                                egui::RichText::new("Flip")
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut data.state.size, 32.0..=1024.0));
                            ui.separator();
                            ui.add(egui::Label::new(
                                egui::RichText::new("Size")
                                    .color(egui::Color32::from_black_alpha(192))
                                    .italics(),
                            ));
                        });

                        let rect = ui
                            .allocate_space(egui::vec2(data.state.size, data.state.size))
                            .1;

                        let painter = ui.painter();

                        let uv =
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

                        let tint = egui::Color32::WHITE;

                        painter.image(texture_id, rect, uv, tint);

                        ui.ctx().request_repaint();
                    });
                },
            );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["oscilloscope"]
    }

    fn make_ui_state(
        &self,
        handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, Color) {
        (
            OscilloscopeUiState {
                buffer_reader: handle.get_buffer_reader(),
                exposure: 5.0,
                gain: 0.7,
                decay: 0.3,
                rotation: 1,
                flip: true,
                size: 512.0,
                prev_sample: (0.0, 0.0),
                image: ColorImage::new([512, 512], Color32::BLACK),
                texture: None,
            },
            Color::default(),
        )
    }
}
