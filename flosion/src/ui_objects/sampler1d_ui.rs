use eframe::egui;

use crate::{
    core::expression::expressionnode::ExpressionNodeWithId,
    objects::sampler1d::Sampler1d,
    ui_core::{
        arguments::ParsedArguments,
        expressiongraphuicontext::ExpressionGraphUiContext,
        expressiongraphuistate::ExpressionGraphUiState,
        expressionobjectui::ExpressionObjectUi,
        expressionodeui::{DisplayStyle, ExpressionNodeUi},
        lexicallayout::lexicallayout::ExpressionNodeLayout,
        object_ui::NoObjectUiState,
    },
};

#[derive(Default)]
pub struct Sampler1dUi {}

impl ExpressionObjectUi for Sampler1dUi {
    type ObjectType = ExpressionNodeWithId<Sampler1d>;
    type StateType = NoObjectUiState;

    fn ui<'a, 'b>(
        &self,
        sampler1d: &mut ExpressionNodeWithId<Sampler1d>,
        _graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        // TODO: custom vertical range

        ExpressionNodeUi::new_named(
            sampler1d.id(),
            "Sampler1d".to_string(),
            DisplayStyle::Framed,
        )
        .show_with(ui, ctx, |ui| {
            let mut values = sampler1d.value().read().to_vec();

            let (id, rect) = ui.allocate_space(egui::vec2(200.0, 100.0));
            let painter = ui.painter();

            painter.rect_filled(rect, egui::Rounding::ZERO, egui::Color32::BLACK);

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

            let r = ui.interact(rect, id, egui::Sense::drag());

            if r.dragged() {
                let p_curr = r.interact_pointer_pos().unwrap();
                let p_prev = p_curr - r.drag_delta();
                let x_curr = ((p_curr.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let x_prev = ((p_prev.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                let t_curr = ((p_curr.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                let t_prev = ((p_prev.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                let v_curr = 1.0 - 2.0 * t_curr;
                let v_prev = 1.0 - 2.0 * t_prev;
                let x0;
                let x1;
                let v0;
                let v1;
                if x_curr <= x_prev {
                    x0 = x_curr;
                    x1 = x_prev;
                    v0 = v_curr;
                    v1 = v_prev;
                } else {
                    x0 = x_prev;
                    x1 = x_curr;
                    v0 = v_prev;
                    v1 = v_curr;
                }
                let i0 = ((x0 * values.len() as f32).floor() as usize).clamp(0, values.len() - 1);
                let i1 = ((x1 * values.len() as f32).ceil() as usize).clamp(0, values.len() - 1);
                let n = i1 - i0;
                for (e, i) in (i0..=i1).enumerate() {
                    let d = (e as f32) / (n as f32).max(1.0);
                    values[i] = v0 + d * (v1 - v0);
                }

                sampler1d.value().write(&values);
            }

            if r.drag_stopped() {
                ctx.request_snapshot();
            }
        });
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["sampler1d"]
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(
        &self,
        _object: &Self::ObjectType,
        _args: ParsedArguments,
    ) -> Result<NoObjectUiState, ()> {
        Ok(NoObjectUiState)
    }
}
