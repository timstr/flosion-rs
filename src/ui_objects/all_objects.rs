use std::collections::HashMap;

use eframe::egui::Ui;

use crate::{
    core::graphobject::{GraphObject, ObjectType, TypedGraphObject},
    ui_core::object_ui::{AnyObjectUi, ObjectUi},
};

use super::{
    dac_ui::DacUi,
    functions_ui::{ConstantUi, UnitSineUi},
    wavegenerator_ui::WaveGeneratorUi,
};

pub struct AllObjectUis {
    mapping: HashMap<ObjectType, Box<dyn AnyObjectUi>>,
}

fn error_ui(ui: &mut Ui, object: &dyn GraphObject, object_type: ObjectType) {
    ui.label(format!(
        "[Unrecognized object type \"{}\" for type {}]",
        object_type.name(),
        object.get_language_type_name()
    ));
}

impl AllObjectUis {
    pub fn new() -> AllObjectUis {
        let mapping: HashMap<ObjectType, Box<dyn AnyObjectUi>> = HashMap::new();
        let mut all_uis = AllObjectUis { mapping };
        all_uis.add::<DacUi>();
        all_uis.add::<WaveGeneratorUi>();
        all_uis.add::<ConstantUi>();
        all_uis.add::<UnitSineUi>();
        all_uis
    }

    pub fn add<T: ObjectUi>(&mut self) {
        self.mapping
            .insert(T::ObjectType::TYPE, Box::new(T::default()));
    }

    pub fn ui(&self, object: &dyn GraphObject, object_type: ObjectType, ui: &mut Ui) {
        match self.mapping.get(&object_type) {
            Some(any_ui) => any_ui.apply(object, ui),
            None => error_ui(ui, object, object_type),
        }
    }
}
