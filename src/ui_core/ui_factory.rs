use std::{collections::HashMap, rc::Rc};

use eframe::egui;
use serialization::Deserializer;

use crate::core::graph::graphobject::{
    GraphObject, GraphObjectHandle, ObjectHandle, ObjectInitialization, ObjectType,
};

use super::{
    arguments::ParsedArguments,
    graph_ui::{GraphUi, GraphUiState},
    object_ui::{AnyObjectUi, ObjectUi},
};

struct ObjectData<G: GraphUi> {
    ui: Rc<dyn AnyObjectUi<G>>,
}

pub struct UiFactory<G: GraphUi> {
    mapping: HashMap<ObjectType, ObjectData<G>>,
}

impl<G: GraphUi> UiFactory<G> {
    pub fn new_empty() -> UiFactory<G> {
        UiFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: ObjectUi<GraphUi = G>>(&mut self) {
        let object_type = <T::HandleType as ObjectHandle<G::Graph>>::ObjectType::get_type();
        self.mapping.insert(
            object_type,
            ObjectData {
                ui: Rc::new(T::default()),
            },
        );
    }

    pub fn all_object_types<'a>(&'a self) -> impl 'a + Iterator<Item = ObjectType> {
        self.mapping.keys().cloned()
    }

    pub fn all_object_uis<'a>(&'a self) -> impl 'a + Iterator<Item = &dyn AnyObjectUi<G>> {
        self.mapping.values().map(|d| &*d.ui)
    }

    pub fn get_object_ui(&self, object_type: ObjectType) -> Rc<dyn AnyObjectUi<G>> {
        Rc::clone(&self.mapping.get(&object_type).unwrap().ui)
    }

    pub fn ui(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        ui_state: &mut G::State,
        ui: &mut egui::Ui,
        ctx: &G::Context<'_>,
        graph: &mut G::Graph,
    ) {
        let object_type = object.get_type();
        let id = object.id();
        match self.mapping.get(&object_type) {
            Some(data) => {
                let state = ui_state.get_object_ui_data(id);
                data.ui.apply(object, &*state, ui_state, ui, ctx, graph);
            }
            None => panic!(
                "Tried to create a ui for an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }

    fn create_state_impl(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        init: ObjectInitialization,
    ) -> Result<G::ObjectUiData, ()> {
        let object_type = object.get_type();
        match self.mapping.get(&object_type) {
            Some(data) => data.ui.make_ui_state(object.id(), object, init),
            None => panic!(
                "Tried to create ui state for an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }

    pub fn create_default_state(&self, object: &GraphObjectHandle<G::Graph>) -> G::ObjectUiData {
        self.create_state_impl(object, ObjectInitialization::Default)
            .unwrap()
    }

    pub fn create_state_from_archive(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        deserializer: Deserializer,
    ) -> Result<G::ObjectUiData, ()> {
        self.create_state_impl(object, ObjectInitialization::Archive(deserializer))
    }

    pub fn create_state_from_arguments(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        arguments: ParsedArguments,
    ) -> Result<G::ObjectUiData, ()> {
        self.create_state_impl(object, ObjectInitialization::Arguments(arguments))
    }
}
