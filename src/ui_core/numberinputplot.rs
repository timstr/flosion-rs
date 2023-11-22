use eframe::egui;

use crate::core::{
    jit::{compilednumberinput::Discretization, server::JitClient},
    number::context::MockNumberContext,
    revision::revision::Revision,
    sound::{soundgraphdata::SoundNumberInputData, soundnumbersource::SoundNumberSourceId},
};

use super::temporallayout::TimeAxis;

enum VerticalRange {
    Automatic,
    // TODO: log plots?
    Linear(std::ops::RangeInclusive<f32>),
}

enum HorizontalDomain {
    Temporal,
    WithRespectTo(SoundNumberSourceId, std::ops::RangeInclusive<f32>),
}

pub struct PlotConfig {
    // TODO: whether to always plot temporally or w.r.t. an input, e.g. wave generator amplitude vs phase
    vertical_range: VerticalRange,
    horizontal_domain: HorizontalDomain,
}

impl PlotConfig {
    pub fn new() -> Self {
        PlotConfig {
            vertical_range: VerticalRange::Automatic,
            horizontal_domain: HorizontalDomain::Temporal,
        }
    }

    pub fn linear_vertical_range(mut self, range: std::ops::RangeInclusive<f32>) -> Self {
        self.vertical_range = VerticalRange::Linear(range);
        self
    }

    pub fn with_respect_to(
        mut self,
        source: SoundNumberSourceId,
        domain: std::ops::RangeInclusive<f32>,
    ) -> Self {
        self.horizontal_domain = HorizontalDomain::WithRespectTo(source, domain);
        self
    }
}

pub(crate) struct NumberInputPlot {
    // TODO: fields for creating mock context?
}

impl NumberInputPlot {
    pub(crate) fn new() -> NumberInputPlot {
        NumberInputPlot {}
    }

    pub(crate) fn show(
        self,
        ui: &mut egui::Ui,
        jit_client: &JitClient,
        ni_data: &SoundNumberInputData,
        time_axis: TimeAxis,
        config: &PlotConfig,
    ) {
        let PlotConfig {
            vertical_range,
            horizontal_domain,
        } = config;
        let compiled_fn =
            jit_client.get_compiled_number_input(ni_data.id(), ni_data.get_revision());
        // TODO: make this configurable / draggable. Where to store such ui state?
        let desired_height = 30.0;
        let desired_width = match horizontal_domain {
            HorizontalDomain::Temporal => ui.available_width(),
            HorizontalDomain::WithRespectTo(_, _) => 100.0,
        };
        let (_, rect) = ui.allocate_space(egui::vec2(desired_width, desired_height));
        let painter = ui.painter();
        painter.rect_filled(rect, egui::Rounding::none(), egui::Color32::BLACK);
        match compiled_fn {
            Some(mut f) => {
                let len = rect.width().floor() as usize;
                let mut dst = Vec::new();
                dst.resize(len, 0.0);
                let number_context = MockNumberContext::new(len);

                let discretization = match horizontal_domain {
                    HorizontalDomain::Temporal => {
                        Discretization::Temporal(time_axis.time_per_x_pixel)
                    }
                    HorizontalDomain::WithRespectTo(_, _) => Discretization::None,
                };

                f.eval(&mut dst, &number_context, discretization);
                let (vmin, vmax) = match vertical_range {
                    VerticalRange::Automatic => {
                        let mut vmin = *dst.first().unwrap();
                        let mut vmax = vmin;
                        for v in dst.iter().cloned() {
                            vmin = v.min(vmin);
                            vmax = v.max(vmax);
                        }
                        (vmin, vmax)
                    }
                    VerticalRange::Linear(range) => (*range.start(), *range.end()),
                };
                debug_assert!(vmax >= vmin);
                // Range spans at least 1e-3 plus 10% extra
                let plot_v_range = 1.1 * (vmax - vmin).max(1e-3);
                let v_middle = 0.5 * (vmin + vmax);
                let plot_vmin = v_middle - 0.5 * plot_v_range;
                let dx = rect.width() / (len - 1) as f32;
                for (i, (v0, v1)) in dst.iter().zip(&dst[1..]).enumerate() {
                    let x0 = rect.left() + i as f32 * dx;
                    let x1 = rect.left() + (i + 1) as f32 * dx;
                    let t0 = ((v0 - plot_vmin) / plot_v_range).clamp(0.0, 1.0);
                    let t1 = ((v1 - plot_vmin) / plot_v_range).clamp(0.0, 1.0);
                    let y0 = rect.bottom() - t0 * rect.height();
                    let y1 = rect.bottom() - t1 * rect.height();
                    painter.line_segment(
                        [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );
                }
                painter.text(
                    rect.left_top(),
                    egui::Align2::LEFT_TOP,
                    format!("{}", vmax),
                    egui::FontId::monospace(8.0),
                    egui::Color32::from_white_alpha(128),
                );
                painter.text(
                    rect.left_bottom(),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{}", vmin),
                    egui::FontId::monospace(8.0),
                    egui::Color32::from_white_alpha(128),
                );
            }
            None => {
                let dot_length: f32 = 10.0;
                let dot_frequency = 4.0;
                let t = dot_frequency * ui.input(|i| i.time);
                let offset = t.fract() as f32 * dot_length;
                let num_dots = (rect.width() / dot_length).ceil() as usize + 1;
                let y = rect.top() + 0.5 * rect.height();
                for i in 0..num_dots {
                    let x0 = rect.left() + offset + ((i as f32 - 1.0) * dot_length);
                    let x1 = x0 + 0.5 * dot_length;
                    let x0 = x0.clamp(rect.left(), rect.right());
                    let x1 = x1.clamp(rect.left(), rect.right());
                    painter.line_segment(
                        [egui::pos2(x0, y), egui::pos2(x1, y)],
                        egui::Stroke::new(2.0, egui::Color32::GRAY),
                    );
                }
                ui.ctx().request_repaint();
            }
        }
        painter.rect_stroke(
            rect,
            egui::Rounding::none(),
            egui::Stroke::new(2.0, egui::Color32::GRAY),
        );
    }
}
