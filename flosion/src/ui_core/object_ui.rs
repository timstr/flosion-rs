use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};

// TODO: this module is misnamed now (any maybe pointless?)

pub struct Color {
    pub color: egui::Color32,
}

impl Default for Color {
    fn default() -> Self {
        Color {
            color: random_object_color(),
        }
    }
}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}
