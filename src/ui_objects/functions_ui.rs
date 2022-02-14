use eframe::egui;

use crate::{
    core::graphobject::ObjectId,
    objects::functions::{Constant, UnitSine},
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct ConstantUi {}

impl ObjectUi for ConstantUi {
    type ObjectType = Constant;
    fn ui(
        &self,
        id: ObjectId,
        object: &Constant,
        graph_state: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_number_source_id().unwrap();
        ObjectWindow::new_number_source(id).show(ui.ctx(), |ui| {
            let mut v = object.get_value();
            let v_old = v;
            ui.label(format!("Constant, value={}", v));
            ui.add(egui::Slider::new(&mut v, 0.0..=1000.0));
            if v != v_old {
                object.set_value(v);
            }
            ui.add(NumberOutputWidget::new(id, graph_state));
        });
    }
}

#[derive(Default)]
pub struct UnitSineUi {}

impl ObjectUi for UnitSineUi {
    type ObjectType = UnitSine;
    fn ui(
        &self,
        id: ObjectId,
        object: &UnitSine,
        graph_state: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_number_source_id().unwrap();
        ObjectWindow::new_number_source(id).show(ui.ctx(), |ui| {
            ui.label("UnitSine");
            ui.add(NumberInputWidget::new(object.input.id(), graph_state));
            ui.add(NumberOutputWidget::new(id, graph_state));
        });
    }
}
