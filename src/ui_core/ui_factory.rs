use std::{cell::RefCell, collections::HashMap, rc::Rc};

use eframe::egui::Ui;

use crate::core::{
    arguments::ParsedArguments,
    graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization, WithObjectType},
    numbersource::PureNumberSource,
    serialization::Deserializer,
    soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
};

use super::{
    graph_ui_state::{GraphUIState, ObjectUiState},
    object_ui::{AnyObjectUi, ObjectUi},
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

    pub fn register_static_sound_processor<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::HandleType as ObjectHandle>::Type: StaticSoundProcessor,
    {
        let name = <<T as ObjectUi>::HandleType as ObjectHandle>::Type::TYPE.name();
        self.mapping.insert(
            name,
            ObjectData {
                ui: Box::new(T::default()),
            },
        );
    }

    pub fn register_dynamic_sound_processor<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::HandleType as ObjectHandle>::Type: DynamicSoundProcessor,
    {
        let name = <<T as ObjectUi>::HandleType as ObjectHandle>::Type::TYPE.name();
        self.mapping.insert(
            name,
            ObjectData {
                ui: Box::new(T::default()),
            },
        );
    }

    pub fn register_number_source<T: ObjectUi>(&mut self)
    where
        <<T as ObjectUi>::HandleType as ObjectHandle>::Type: PureNumberSource,
    {
        let name = <<T as ObjectUi>::HandleType as ObjectHandle>::Type::TYPE.name();
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

    // NOTE: Arc is used since some UI objects may want to store an Arc to the graph object,
    // e.g. in a callback function that is passed elsewhere
    pub fn ui(&self, object: &GraphObjectHandle, graph_state: &mut GraphUIState, ui: &mut Ui) {
        let name = object.get_type().name();
        let id = object.id();
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
        object: &GraphObjectHandle,
        init: ObjectInitialization,
    ) -> Result<Rc<RefCell<dyn ObjectUiState>>, ()> {
        let name = object.get_type().name();
        match self.mapping.get(name) {
            Some(data) => data.ui.make_ui_state(object, init),
            None => panic!(
                "Tried to create ui state for an object of unrecognized type \"{}\"",
                name
            ),
        }
    }

    pub fn create_default_state(
        &self,
        object: &GraphObjectHandle,
    ) -> Rc<RefCell<dyn ObjectUiState>> {
        self.create_state_impl(object, ObjectInitialization::Default)
            .unwrap()
    }

    pub fn create_state_from_args(
        &self,
        object: &GraphObjectHandle,
        args: &ParsedArguments,
    ) -> Rc<RefCell<dyn ObjectUiState>> {
        self.create_state_impl(object, ObjectInitialization::Args(args))
            .unwrap()
    }

    pub fn create_state_from_archive(
        &self,
        object: &GraphObjectHandle,
        deserializer: Deserializer,
    ) -> Result<Rc<RefCell<dyn ObjectUiState>>, ()> {
        self.create_state_impl(object, ObjectInitialization::Archive(deserializer))
    }
}
