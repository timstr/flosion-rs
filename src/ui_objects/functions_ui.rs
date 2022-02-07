use eframe::egui;

use crate::{
    objects::functions::{Constant, UnitSine},
    ui_core::object_ui::ObjectUi,
};

#[derive(Default)]
pub struct ConstantUi {}

impl ObjectUi for ConstantUi {
    type ObjectType = Constant;
    fn ui(&self, object: &Constant, ui: &mut eframe::egui::Ui) {
        let mut v = object.get_value();
        let v_old = v;
        ui.label(format!("Constant Ui, value={}", v));
        ui.add(egui::Slider::new(&mut v, 0.0..=1000.0));
        if v != v_old {
            object.set_value(v);
        }
    }
}

#[derive(Default)]
pub struct UnitSineUi {}

impl ObjectUi for UnitSineUi {
    type ObjectType = UnitSine;
    fn ui(&self, _object: &UnitSine, ui: &mut eframe::egui::Ui) {
        ui.label("UnitSine Ui");
    }
}
