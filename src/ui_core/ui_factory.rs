use std::{cell::RefCell, collections::HashMap};

use eframe::egui;

use crate::core::{
    arguments::ParsedArguments,
    serialization::Deserializer,
    sound::{
        graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization, WithObjectType},
        soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
    },
};

use super::{
    graph_ui_state::GraphUIState,
    object_ui::{AnyObjectUi, ObjectUi},
    object_ui_states::AnyObjectUiState,
    ui_context::UiContext,
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
        ui: &mut egui::Ui,
        ctx: &UiContext,
    ) {
        let name = object.get_type().name();
        let id = object.id();
        match self.mapping.get(name) {
            Some(data) => {
                let state = ctx.object_states().get_object_data(id);
                data.ui.apply(object, state, graph_state, ui, ctx);
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
    ) -> Result<Box<RefCell<dyn AnyObjectUiState>>, ()> {
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
    ) -> Box<RefCell<dyn AnyObjectUiState>> {
        self.create_state_impl(object, ObjectInitialization::Default)
            .unwrap()
    }

    pub fn create_state_from_args(
        &self,
        object: &GraphObjectHandle,
        args: &ParsedArguments,
    ) -> Box<RefCell<dyn AnyObjectUiState>> {
        self.create_state_impl(object, ObjectInitialization::Args(args))
            .unwrap()
    }

    pub fn create_state_from_archive(
        &self,
        object: &GraphObjectHandle,
        deserializer: Deserializer,
    ) -> Result<Box<RefCell<dyn AnyObjectUiState>>, ()> {
        self.create_state_impl(object, ObjectInitialization::Archive(deserializer))
    }
}
