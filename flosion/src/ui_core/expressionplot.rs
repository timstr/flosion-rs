use eframe::egui;

use crate::core::{
    expression::expressiongraph::ExpressionGraph,
    jit::{
        cache::JitCache,
        compiledexpression::{CompiledExpressionFunction, Discretization},
        jit::{ExpressionTestDomain, Interval, JitMode},
    },
    sound::{
        argument::ProcessorArgumentLocation,
        expression::{ExpressionParameterMapping, ProcessorExpressionLocation},
    },
};

use super::{soundgraphuinames::SoundGraphUiNames, stackedlayout::timeaxis::TimeAxis};

enum VerticalRange {
    Automatic,
    // TODO: log plots?
    Linear(std::ops::RangeInclusive<f32>),
}

pub struct PlotConfig {
    // TODO: whether to always plot temporally or w.r.t. an input, e.g. wave generator amplitude vs phase
    vertical_range: VerticalRange,
    horizontal_domain: ExpressionTestDomain,
}

impl PlotConfig {
    pub fn new() -> Self {
        PlotConfig {
            vertical_range: VerticalRange::Automatic,
            horizontal_domain: ExpressionTestDomain::Temporal,
        }
    }

    pub fn linear_vertical_range(mut self, range: std::ops::RangeInclusive<f32>) -> Self {
        self.vertical_range = VerticalRange::Linear(range);
        self
    }

    pub fn with_respect_to(
        mut self,
        arg: ProcessorArgumentLocation,
        domain: std::ops::RangeInclusive<f32>,
    ) -> Self {
        self.horizontal_domain = ExpressionTestDomain::WithRespectTo(
            arg,
            Interval::Linear {
                from: *domain.start(),
                to: *domain.end(),
            },
        );
        self
    }
}

pub(crate) struct ExpressionPlot {
    // TODO: fields for creating mock context?
}

impl ExpressionPlot {
    pub(crate) fn new() -> ExpressionPlot {
        ExpressionPlot {}
    }

    pub(crate) fn show(
        self,
        ui: &mut egui::Ui,
        jit_cache: &JitCache,
        location: ProcessorExpressionLocation,
        expr_graph: &ExpressionGraph,
        mapping: &ExpressionParameterMapping,
        time_axis: TimeAxis,
        config: &PlotConfig,
        names: &SoundGraphUiNames,
    ) {
        let PlotConfig {
            vertical_range,
            horizontal_domain,
        } = config;
        let compiled_fn = jit_cache.request_compiled_expression(
            location,
            expr_graph,
            mapping,
            JitMode::Test(config.horizontal_domain),
        );
        // TODO: make this configurable / draggable. Where to store such ui state?
        let desired_height = 30.0;
        let desired_width = match horizontal_domain {
            ExpressionTestDomain::Temporal => ui.available_width(),
            ExpressionTestDomain::WithRespectTo(_, _) => 100.0,
        };
        let (_, rect) = ui.allocate_space(egui::vec2(desired_width, desired_height));
        ui.painter()
            .rect_filled(rect, egui::Rounding::ZERO, egui::Color32::BLACK);

        match compiled_fn {
            Some(compiled_fn) => {
                self.plot_compiled_function(
                    ui,
                    compiled_fn,
                    rect,
                    horizontal_domain,
                    vertical_range,
                    time_axis,
                    names,
                );
            }
            None => {
                self.plot_missing_function(ui, rect);
            }
        }

        ui.painter().rect_stroke(
            rect,
            egui::Rounding::ZERO,
            egui::Stroke::new(2.0, egui::Color32::GRAY),
        );
    }

    fn plot_compiled_function(
        &self,
        ui: &mut egui::Ui,
        mut compiled_fn: CompiledExpressionFunction,
        rect: egui::Rect,
        horizontal_domain: &ExpressionTestDomain,
        vertical_range: &VerticalRange,
        time_axis: TimeAxis,
        names: &SoundGraphUiNames,
    ) {
        let len = rect.width().floor() as usize;
        let mut dsts: Vec<Vec<f32>> = Vec::new();
        dsts.resize_with(compiled_fn.num_destination_arrays(), || {
            let mut v = Vec::new();
            v.resize(len, 0.0);
            v
        });

        let mut dst_slices: Vec<&mut [f32]> = dsts.iter_mut().map(|v| &mut v[..]).collect();

        let discretization = match horizontal_domain {
            ExpressionTestDomain::Temporal => {
                Discretization::Temporal(time_axis.seconds_per_x_pixel)
            }
            ExpressionTestDomain::WithRespectTo(_, _) => Discretization::None,
        };

        compiled_fn.eval_in_test_mode(&mut dst_slices, discretization);

        let (vmin, vmax) = match vertical_range {
            VerticalRange::Automatic => {
                let mut vmin = f32::INFINITY;
                let mut vmax = f32::NEG_INFINITY;
                for dst in &dsts {
                    for &v in dst {
                        if v.is_finite() {
                            vmin = vmin.min(v);
                            vmax = vmax.max(v);
                        }
                    }
                }
                (
                    if vmin.is_finite() { vmin } else { 0.0 },
                    if vmax.is_finite() { vmax } else { 0.0 },
                )
            }
            VerticalRange::Linear(range) => (*range.start(), *range.end()),
        };
        // Range spans at least 1e-3 plus 10% extra
        let plot_v_range = 1.1 * (vmax - vmin).max(1e-3);
        let v_middle = 0.5 * (vmin + vmax);
        let plot_vmin = v_middle - 0.5 * plot_v_range;
        let dx = rect.width() / (len - 1) as f32;

        // TODO: different colours for different arrays?
        // And match those colours to colours in the expression ui?
        for dst in dsts {
            for (i, (v0, v1)) in dst.iter().zip(&dst[1..]).enumerate() {
                let x0 = rect.left() + i as f32 * dx;
                let x1 = rect.left() + (i + 1) as f32 * dx;
                if v0.is_finite() && v1.is_finite() {
                    let t0 = ((v0 - plot_vmin) / plot_v_range).clamp(0.0, 1.0);
                    let t1 = ((v1 - plot_vmin) / plot_v_range).clamp(0.0, 1.0);
                    let y0 = rect.bottom() - t0 * rect.height();
                    let y1 = rect.bottom() - t1 * rect.height();
                    ui.painter().line_segment(
                        [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                        egui::Stroke::new(2.0, egui::Color32::from_white_alpha(128)),
                    );
                } else {
                    ui.painter().line_segment(
                        [egui::pos2(x1, rect.top()), egui::pos2(x1, rect.bottom())],
                        egui::Stroke::new(
                            2.0,
                            egui::Color32::from_rgba_unmultiplied(
                                255,
                                0,
                                0,
                                if i % 4 == 0 { 255 } else { 64 },
                            ),
                        ),
                    );
                }
            }
        }

        let font_id = egui::FontId::monospace(10.0);

        // Write the vertical max at the top left
        ui.painter().text(
            rect.left_top(),
            egui::Align2::LEFT_TOP,
            format!("{}", vmax),
            font_id.clone(),
            egui::Color32::from_white_alpha(128),
        );

        // Write the vertical min at the bottom left
        ui.painter().text(
            rect.left_bottom(),
            egui::Align2::LEFT_BOTTOM,
            format!("{}", vmin),
            font_id.clone(),
            egui::Color32::from_white_alpha(128),
        );

        match horizontal_domain {
            ExpressionTestDomain::Temporal => {
                // Plotting against time is implicit. The plot will already extend to the full
                // width of and line up with other temporal queues in the layout.
            }
            ExpressionTestDomain::WithRespectTo(arg_id, domain) => {
                // If not plotting against time, write the extent and domain at the bottom.
                let domain_rect = egui::Rect::from_x_y_ranges(
                    rect.left()..=rect.right(),
                    (rect.bottom() + 3.0)..=(rect.bottom() + 13.0),
                );
                ui.allocate_rect(domain_rect, egui::Sense::hover());

                let (domain_start, domain_end) = match domain {
                    Interval::Linear { from, to } => (from, to),
                };

                // write domain min at left
                ui.painter().text(
                    egui::pos2(domain_rect.left() + 5.0, domain_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", domain_start),
                    font_id.clone(),
                    egui::Color32::from_white_alpha(128),
                );

                // write domain max at right
                ui.painter().text(
                    egui::pos2(domain_rect.right() - 5.0, domain_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}", domain_end),
                    font_id.clone(),
                    egui::Color32::from_white_alpha(128),
                );

                // write arg name at center
                ui.painter().text(
                    domain_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    names
                        .argument(*arg_id)
                        .iter()
                        .cloned()
                        .next()
                        .unwrap_or("???"),
                    font_id.clone(),
                    egui::Color32::from_white_alpha(128),
                );

                // draw tick marks left and right
                let tick_stroke = egui::Stroke::new(2.0, egui::Color32::from_white_alpha(32));
                ui.painter().line_segment(
                    [domain_rect.left_top(), domain_rect.left_bottom()],
                    tick_stroke,
                );
                ui.painter().line_segment(
                    [domain_rect.right_top(), domain_rect.right_bottom()],
                    tick_stroke,
                );
            }
        }
    }

    fn plot_missing_function(&self, ui: &mut egui::Ui, rect: egui::Rect) {
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
            ui.painter().line_segment(
                [egui::pos2(x0, y), egui::pos2(x1, y)],
                egui::Stroke::new(2.0, egui::Color32::GRAY),
            );
        }
        ui.ctx().request_repaint();
    }
}
