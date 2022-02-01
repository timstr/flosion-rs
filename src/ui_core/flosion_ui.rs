use eframe::{egui, epi};

#[derive(Default)]
pub struct FlosionApp {}

impl epi::App for FlosionApp {
    fn update(&mut self, ctx: &egui::CtxRef, _frame: &epi::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hi earthguy");
            egui::Area::new("my area").show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::DARK_BLUE)
                    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                    .margin(egui::Vec2::splat(10.0))
                    .show(ui, |ui| {
                        if ui.label("label inside area").dragged() {
                            println!("I'm being dragged!");
                        }
                    });
            });
        });
    }

    fn name(&self) -> &str {
        "Flosion"
    }
}
