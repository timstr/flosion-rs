use chive::{Chivable, ChiveIn, ChiveOut};
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

impl Chivable for Color {
    fn chive_in(&self, chive_in: &mut ChiveIn) {
        chive_in.u32(u32::from_be_bytes(self.color.to_array()))
    }

    fn chive_out(chive_out: &mut ChiveOut) -> Result<Self, ()> {
        let i = chive_out.u32()?;
        let [r, g, b, a] = i.to_be_bytes();
        Ok(Color {
            color: egui::Color32::from_rgba_premultiplied(r, g, b, a),
        })
    }
}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}
