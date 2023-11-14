use eframe::egui;

use crate::core::{
    jit::server::JitClient, number::context::MockNumberContext, revision::revision::Revision,
    sound::soundgraphdata::SoundNumberInputData,
};

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
    ) {
        let compiled_fn =
            jit_client.get_compiled_number_input(ni_data.id(), ni_data.get_revision());
        // TODO: make this configurable / draggable
        let desired_height = 20.0;
        let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), desired_height));
        let painter = ui.painter();
        painter.rect_filled(rect, egui::Rounding::none(), egui::Color32::BLACK);
        match compiled_fn {
            Some(f) => {
                // TODO: use temporal layout to inform time span
                let len = rect.width().floor() as usize;
                let mut dst = Vec::new();
                dst.resize(len, 0.0);
                let number_context = MockNumberContext::new(len);
                f.eval(&mut dst, &number_context);
                let dx = rect.width() / (len - 1) as f32;
                for (i, (v0, v1)) in dst.iter().zip(&dst[1..]).enumerate() {
                    let x0 = rect.left() + i as f32 * dx;
                    let x1 = rect.left() + (i + 1) as f32 * dx;
                    let y0 = rect.top() + rect.height() * (0.5 - v0.clamp(-1.0, 1.0) * 0.5);
                    let y1 = rect.top() + rect.height() * (0.5 - v1.clamp(-1.0, 1.0) * 0.5);
                    painter.line_segment(
                        [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );
                }
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
