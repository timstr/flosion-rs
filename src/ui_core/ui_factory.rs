use std::collections::HashMap;

use eframe::egui;

use crate::core::{
    arguments::ParsedArguments,
    graph::graphobject::{GraphObject, GraphObjectHandle, ObjectHandle, ObjectInitialization},
    serialization::Deserializer,
};

use super::{
    graph_ui::{GraphUi, GraphUiContext},
    object_ui::{AnyObjectUi, ObjectUi},
    object_ui_states::AnyObjectUiState,
};

struct ObjectData<G: GraphUi> {
    ui: Box<dyn AnyObjectUi<G>>,
}

pub struct UiFactory<G: GraphUi> {
    mapping: HashMap<&'static str, ObjectData<G>>,
}

impl<G: GraphUi> UiFactory<G> {
    pub fn new_empty() -> UiFactory<G> {
        UiFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: ObjectUi<GraphUi = G>>(&mut self) {
        let name = <T::HandleType as ObjectHandle<G::Graph>>::ObjectType::get_type().name();
        self.mapping.insert(
            name,
            ObjectData {
                ui: Box::new(T::default()),
            },
        );
    }

    // pub fn register_static_sound_processor<T: ObjectUi>(&mut self)
    // where
    //     <<T as ObjectUi>::HandleType as ObjectHandle>::Type: StaticSoundProcessor,
    // {
    //     let name = <<T as ObjectUi>::HandleType as ObjectHandle>::Type::TYPE.name();
    //     self.mapping.insert(
    //         name,
    //         ObjectData {
    //             ui: Box::new(T::default()),
    //         },
    //     );
    // }

    // pub fn register_dynamic_sound_processor<T: ObjectUi>(&mut self)
    // where
    //     <<T as ObjectUi>::HandleType as ObjectHandle>::Type: DynamicSoundProcessor,
    // {
    //     let name = <<T as ObjectUi>::HandleType as ObjectHandle>::Type::TYPE.name();
    //     self.mapping.insert(
    //         name,
    //         ObjectData {
    //             ui: Box::new(T::default()),
    //         },
    //     );
    // }

    pub fn all_object_types(&self) -> impl Iterator<Item = &str> {
        self.mapping.keys().cloned()
    }

    pub fn get_object_ui(&self, object_type_str: &str) -> &dyn AnyObjectUi<G> {
        &*self.mapping.get(object_type_str).unwrap().ui
    }

    pub fn ui(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        graph_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
    ) {
        let name = object.get_type().name();
        let id = object.id();
        match self.mapping.get(name) {
            Some(data) => {
                let state = ctx.get_object_ui_data(id);
                data.ui
                    .apply(object, &mut state.borrow_mut(), graph_state, ui, ctx);
            }
            None => panic!(
                "Tried to create a ui for an object of unrecognized type \"{}\"",
                name
            ),
        }
    }

    fn create_state_impl(
        &self,
        object: &GraphObjectHandle<G::Graph>,
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

    pub fn create_default_state(
        &self,
        object: &GraphObjectHandle<G::Graph>,
    ) -> Box<dyn AnyObjectUiState> {
        self.create_state_impl(object, ObjectInitialization::Default)
            .unwrap()
    }

    pub fn create_state_from_args(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        args: &ParsedArguments,
    ) -> Box<dyn AnyObjectUiState> {
        self.create_state_impl(object, ObjectInitialization::Args(args))
            .unwrap()
    }

    pub fn create_state_from_archive(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        deserializer: Deserializer,
    ) -> Result<Box<dyn AnyObjectUiState>, ()> {
        self.create_state_impl(object, ObjectInitialization::Archive(deserializer))
    }
}
