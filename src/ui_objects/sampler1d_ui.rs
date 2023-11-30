use eframe::egui;

use crate::{
    core::number::{numbergraph::NumberGraph, numbersource::StatefulNumberSourceHandle},
    objects::sampler1d::Sampler1d,
    ui_core::{
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::{DisplayStyle, NumberSourceUi},
        object_ui::ObjectUi,
    },
};

#[derive(Default)]
pub struct Sampler1dUi {}

impl ObjectUi for Sampler1dUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulNumberSourceHandle<Sampler1d>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        sampler1d: StatefulNumberSourceHandle<Sampler1d>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut NumberGraph,
    ) {
        // TODO: detect drags, edit samples
        // TODO: custom vertical range

        NumberSourceUi::new_named(
            sampler1d.id(),
            "Sampler1d".to_string(),
            DisplayStyle::Framed,
        )
        .show_with(ui, ctx, ui_state, |ui, ui_state| {
            let values = sampler1d.value().read().to_vec();

            let (_, rect) = ui.allocate_space(egui::vec2(100.0, 50.0));
            let painter = ui.painter();

            painter.rect_filled(rect, egui::Rounding::none(), egui::Color32::BLACK);

            let dx = rect.width() / (values.len() - 1) as f32;
            for (i, (v0, v1)) in values.iter().zip(&values[1..]).enumerate() {
                let x0 = rect.left() + i as f32 * dx;
                let x1 = rect.left() + (i + 1) as f32 * dx;
                // HACK assuming range of -1 to 1
                let t0 = (0.5 * (*v0 + 1.0)).clamp(0.0, 1.0);
                let t1 = (0.5 * (*v1 + 1.0)).clamp(0.0, 1.0);
                let y0 = rect.bottom() - t0 * rect.height();
                let y1 = rect.bottom() - t1 * rect.height();
                painter.line_segment(
                    [egui::pos2(x0, y0), egui::pos2(x1, y1)],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );
            }
        });
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["sampler1d"]
    }
}
