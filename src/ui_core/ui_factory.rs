use std::{cell::RefCell, collections::HashMap, rc::Rc};

use eframe::egui::Ui;

use crate::core::{
    arguments::ParsedArguments,
    graphobject::{GraphObject, TypedGraphObject, WithObjectType},
    numbersource::PureNumberSource,
    soundprocessor::SoundProcessor,
};

use super::{
    graph_ui_state::{GraphUIState, ObjectUiState},
    object_ui::{AnyObjectUi, ObjectUi, UiInitialization},
};

struct ObjectData {
    ui: Box<dyn AnyObjectUi>,
}

pub struct UiFactory {
    mapping: HashMap<&'static str, ObjectData>,
}

impl UiFactory {
    pub fn new_empty() -> UiFactory {
        UiFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register_sound_processor<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::WrapperType as TypedGraphObject>::Type: SoundProcessor,
    {
        let name = <<T as ObjectUi>::WrapperType as TypedGraphObject>::Type::TYPE.name();
        self.mapping.insert(
            name,
            ObjectData {
                ui: Box::new(T::default()),
            },
        );
    }

    pub fn register_number_source<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::WrapperType as TypedGraphObject>::Type: PureNumberSource,
    {
        let name = <<T as ObjectUi>::WrapperType as TypedGraphObject>::Type::TYPE.name();
        self.mapping.insert(
            name,
            ObjectData {
                ui: Box::new(T::default()),
            },
        );
    }

    pub fn all_object_types(&self) -> impl Iterator<Item = &str> {
        self.mapping.keys().cloned()
    }

    pub fn get_object_ui(&self, object_type_str: &str) -> &dyn AnyObjectUi {
        &*self.mapping.get(object_type_str).unwrap().ui
    }

    pub fn ui(&self, object: &dyn GraphObject, graph_state: &mut GraphUIState, ui: &mut Ui) {
        let name = object.get_type().name();
        let id = object.get_id();
        match self.mapping.get(name) {
            Some(data) => {
                let state_rc = graph_state.get_object_state(id);
                let state_ref = state_rc.borrow();
                data.ui.apply(id, object, &*state_ref, graph_state, ui);
            }
            None => panic!(
                "Tried to create a ui for an object of unrecognized type \"{}\"",
                name
            ),
        }
    }

    fn create_state_impl(
        &self,
        object: &dyn GraphObject,
        init: UiInitialization,
    ) -> Rc<RefCell<dyn ObjectUiState>> {
        let name = object.get_type().name();
        match self.mapping.get(name) {
            Some(data) => data.ui.make_ui_state(object, init),
            None => panic!(
                "Tried to create ui state for an object of unrecognized type \"{}\"",
                name
            ),
        }
    }

    pub fn create_default_state(&self, object: &dyn GraphObject) -> Rc<RefCell<dyn ObjectUiState>> {
        self.create_state_impl(object, UiInitialization::Default)
    }

    pub fn create_state_from_args(
        &self,
        object: &dyn GraphObject,
        args: &ParsedArguments,
    ) -> Rc<RefCell<dyn ObjectUiState>> {
        self.create_state_impl(object, UiInitialization::Args(args))
    }
}
