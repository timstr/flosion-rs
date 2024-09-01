use eframe::egui;

use crate::core::expression::expressionnode::ExpressionNodeId;

use super::expressiongraphuicontext::ExpressionGraphUiContext;

pub enum DisplayStyle {
    Framed,
    Frameless,
}

pub struct ExpressionNodeUi {
    _node_id: ExpressionNodeId,
    label: Option<String>,
    display_style: DisplayStyle,
}

impl ExpressionNodeUi {
    pub fn new_named(
        node_id: ExpressionNodeId,
        label: String,
        display_style: DisplayStyle,
    ) -> ExpressionNodeUi {
        ExpressionNodeUi {
            _node_id: node_id,
            label: Some(label),
            display_style,
        }
    }

    pub fn new_unnamed(node_id: ExpressionNodeId, display_style: DisplayStyle) -> ExpressionNodeUi {
        ExpressionNodeUi {
            _node_id: node_id,
            label: None,
            display_style,
        }
    }

    pub fn show(self, ui: &mut egui::Ui, ctx: &ExpressionGraphUiContext) {
        self.show_with(ui, ctx, |_ui| {});
    }

    pub fn show_with<F: FnOnce(&mut egui::Ui)>(
        self,
        ui: &mut egui::Ui,
        _ctx: &ExpressionGraphUiContext,
        add_contents: F,
    ) {
        ui.spacing_mut().item_spacing.x = 3.0;
        let show_impl = |ui: &mut egui::Ui| {
            if let Some(label) = self.label {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(label)
                            .color(egui::Color32::WHITE)
                            .strong(),
                    )
                    .sense(egui::Sense::click())
                    .wrap(false),
                );
            }
            add_contents(ui);
        };

        match self.display_style {
            DisplayStyle::Framed => {
                let frame = egui::Frame::default()
                    .fill(egui::Color32::from_rgb(0, 128, 0))
                    .rounding(5.0)
                    .inner_margin(2.0);
                frame.show(ui, show_impl);
            }
            DisplayStyle::Frameless => {
                let frame = egui::Frame::default()
                    .fill(egui::Color32::BLACK)
                    .rounding(2.0)
                    .inner_margin(1.0);
                frame.show(ui, show_impl);
            }
        }
    }
}
