use std::collections::HashMap;

use eframe::egui::Ui;

use crate::core::{
    arguments::ParsedArguments,
    graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization, WithObjectType},
    numbersource::PureNumberSource,
    serialization::Deserializer,
    soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
};

use super::{
    graph_ui_state::GraphUIState,
    object_ui::{AnyObjectUi, ObjectUi},
    object_ui_states::{AnyObjectUiState, ObjectUiStates},
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

    pub fn ui(
        &self,
        object: &GraphObjectHandle,
        graph_state: &mut GraphUIState,
        object_states: &mut ObjectUiStates,
        ui: &mut Ui,
    ) {
        let name = object.get_type().name();
        let id = object.id();
        match self.mapping.get(name) {
            Some(data) => {
                let state = object_states.get_object_data(id);
                data.ui.apply(object, state, graph_state, ui);
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
    ) -> Result<Box<dyn AnyObjectUiState>, ()> {
        let name = object.get_type().name();
        match self.mapping.get(name) {
            Some(data) => data.ui.make_ui_state(object, init),
            None => panic!(
                "Tried to create ui state for an object of unrecognized type \"{}\"",
                name
            ),
        }
    }

    pub fn create_default_state(&self, object: &GraphObjectHandle) -> Box<dyn AnyObjectUiState> {
        self.create_state_impl(object, ObjectInitialization::Default)
            .unwrap()
    }

    pub fn create_state_from_args(
        &self,
        object: &GraphObjectHandle,
        args: &ParsedArguments,
    ) -> Box<dyn AnyObjectUiState> {
        self.create_state_impl(object, ObjectInitialization::Args(args))
            .unwrap()
    }

    pub fn create_state_from_archive(
        &self,
        object: &GraphObjectHandle,
        deserializer: Deserializer,
    ) -> Result<Box<dyn AnyObjectUiState>, ()> {
        self.create_state_impl(object, ObjectInitialization::Archive(deserializer))
    }
}
