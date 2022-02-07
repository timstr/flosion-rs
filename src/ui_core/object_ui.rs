use std::any::type_name;

use eframe::egui::Ui;

use crate::core::graphobject::{GraphObject, TypedGraphObject};

pub trait ObjectUi: 'static + Default {
    type ObjectType: TypedGraphObject;
    fn ui(&self, object: &Self::ObjectType, ui: &mut Ui);
}

pub trait AnyObjectUi {
    fn apply(&self, object: &dyn GraphObject, ui: &mut Ui);
}

impl<T: ObjectUi> AnyObjectUi for T {
    fn apply(&self, object: &dyn GraphObject, ui: &mut Ui) {
        let any = object.as_any();
        debug_assert!(
            any.is::<T::ObjectType>(),
            "AnyObjectUi expected to receive type {}, but got {} instead",
            type_name::<T::ObjectType>(),
            object.get_language_type_name()
        );
        let dc_object = any.downcast_ref::<T::ObjectType>().unwrap();
        self.ui(dc_object, ui);
    }
}
