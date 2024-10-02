use eframe::egui;

use crate::core::{
    expression::{context::MockExpressionContext, expressiongraph::ExpressionGraph},
    jit::{
        cache::JitCache,
        compiledexpression::{CompiledExpressionFunction, Discretization},
    },
    sound::{
        expression::ProcessorExpressionLocation, expressionargument::SoundExpressionArgumentId,
        soundgraph::SoundGraph, soundgraphdata::ExpressionParameterMapping,
    },
};

use super::{soundgraphuinames::SoundGraphUiNames, stackedlayout::timeaxis::TimeAxis};

enum VerticalRange {
    Automatic,
    // TODO: log plots?
    Linear(std::ops::RangeInclusive<f32>),
}

enum HorizontalDomain {
    Temporal,
    WithRespectTo(SoundExpressionArgumentId, std::ops::RangeInclusive<f32>),
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
        source: SoundExpressionArgumentId,
        domain: std::ops::RangeInclusive<f32>,
    ) -> Self {
        self.horizontal_domain = HorizontalDomain::WithRespectTo(source, domain);
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
        graph: &SoundGraph,
    ) {
        let PlotConfig {
            vertical_range,
            horizontal_domain,
        } = config;
        let compiled_fn = jit_cache.get_compiled_expression(location, expr_graph, &mapping, graph);
        // TODO: make this configurable / draggable. Where to store such ui state?
        let desired_height = 30.0;
        let desired_width = match horizontal_domain {
            HorizontalDomain::Temporal => ui.available_width(),
            HorizontalDomain::WithRespectTo(_, _) => 100.0,
        };
        let (_, rect) = ui.allocate_space(egui::vec2(desired_width, desired_height));
        ui.painter()
            .rect_filled(rect, egui::Rounding::ZERO, egui::Color32::BLACK);

        self.plot_compiled_function(
            ui,
            compiled_fn,
            rect,
            horizontal_domain,
            vertical_range,
            time_axis,
            names,
        );

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
        horizontal_domain: &HorizontalDomain,
        vertical_range: &VerticalRange,
        time_axis: TimeAxis,
        names: &SoundGraphUiNames,
    ) {
        let len = rect.width().floor() as usize;
        let mut dst = Vec::new();
        dst.resize(len, 0.0);
        let expr_context = MockExpressionContext::new(len);

        let discretization = match horizontal_domain {
            HorizontalDomain::Temporal => Discretization::Temporal(time_axis.time_per_x_pixel),
            HorizontalDomain::WithRespectTo(_, _) => Discretization::None,
        };

        compiled_fn.eval(&mut dst, &expr_context, discretization);

        let (vmin, vmax) = match vertical_range {
            VerticalRange::Automatic => {
                let first_val = *dst.first().unwrap();
                let mut vmin = if first_val.is_finite() {
                    first_val
                } else {
                    0.0
                };
                let mut vmax = vmin;
                for v in dst.iter().cloned() {
                    if v.is_finite() {
                        vmin = v.min(vmin);
                        vmax = v.max(vmax);
                    }
                }
                (vmin, vmax)
            }
            VerticalRange::Linear(range) => (*range.start(), *range.end()),
        };
        // Range spans at least 1e-3 plus 10% extra
        let plot_v_range = 1.1 * (vmax - vmin).max(1e-3);
        let v_middle = 0.5 * (vmin + vmax);
        let plot_vmin = v_middle - 0.5 * plot_v_range;
        let dx = rect.width() / (len - 1) as f32;

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
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
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
            HorizontalDomain::Temporal => {
                // Plotting against time is implicit. The plot will already extend to the full
                // width of and line up with other temporal queues in the layout.
            }
            HorizontalDomain::WithRespectTo(arg_id, domain) => {
                // If not plotting against time, write the extent and domain at the bottom.
                let domain_rect = egui::Rect::from_x_y_ranges(
                    rect.left()..=rect.right(),
                    (rect.bottom() + 3.0)..=(rect.bottom() + 13.0),
                );
                ui.allocate_rect(domain_rect, egui::Sense::hover());

                // write domain min at left
                ui.painter().text(
                    egui::pos2(domain_rect.left() + 5.0, domain_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}", domain.start()),
                    font_id.clone(),
                    egui::Color32::from_white_alpha(128),
                );

                // write domain max at right
                ui.painter().text(
                    egui::pos2(domain_rect.right() - 5.0, domain_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}", domain.end()),
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
                        .map(|n| n.name())
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
}
