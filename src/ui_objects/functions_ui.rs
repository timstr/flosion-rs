use eframe::egui;

use crate::{
    core::graphobject::ObjectId,
    objects::functions::{Constant, Sine, UnitSine},
    ui_core::{
        arguments::{ArgumentList, ArgumentValue, ParsedArguments},
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

    fn arguments(&self) -> ArgumentList {
        let mut args = ArgumentList::new();
        args.add("value", ArgumentValue::Float(0.0));
        args
    }

    fn init_object(&self, object: &Constant, args: ParsedArguments) {
        object.set_value(args.get("value").as_float());
    }
}

#[derive(Default)]
pub struct SineUi {}

impl ObjectUi for SineUi {
    type ObjectType = Sine;
    fn ui(
        &self,
        id: ObjectId,
        object: &Sine,
        graph_state: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
    ) {
        let id = id.as_number_source_id().unwrap();
        ObjectWindow::new_number_source(id).show(ui.ctx(), |ui| {
            ui.label("Sine");
            ui.add(NumberInputWidget::new(object.input.id(), graph_state));
            ui.add(NumberOutputWidget::new(id, graph_state));
        });
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["sin"]
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

    fn aliases(&self) -> &'static [&'static str] {
        &["usin"]
    }
}
