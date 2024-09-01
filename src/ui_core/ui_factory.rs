use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

use chive::ChiveOut;
use eframe::egui;

use crate::core::graph::graphobject::{GraphObject, GraphObjectHandle, ObjectHandle, ObjectType};

use super::{
    arguments::ParsedArguments,
    graph_ui::{GraphUi, GraphUiState},
    object_ui::{AnyObjectUi, ObjectUi},
};

struct ObjectData<G: GraphUi> {
    ui: Box<dyn AnyObjectUi<G>>,
}

pub struct UiFactory<G: GraphUi> {
    mapping: HashMap<ObjectType, ObjectData<G>>,
}

impl<G: GraphUi> UiFactory<G> {
    pub(crate) fn new_empty() -> UiFactory<G> {
        UiFactory {
            mapping: HashMap::new(),
        }
    }

    pub(crate) fn register<T: ObjectUi<GraphUi = G>>(&mut self) {
        let object_type = <T::HandleType as ObjectHandle<G::Graph>>::ObjectType::get_type();
        self.mapping.insert(
            object_type,
            ObjectData {
                ui: Box::new(T::default()),
            },
        );
    }

    pub(crate) fn all_object_uis<'a>(&'a self) -> impl 'a + Iterator<Item = &dyn AnyObjectUi<G>> {
        self.mapping.values().map(|d| &*d.ui)
    }

    pub(crate) fn get_object_ui(&self, object_type: ObjectType) -> &dyn AnyObjectUi<G> {
        &*self.mapping.get(&object_type).unwrap().ui
    }

    pub(crate) fn ui(
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
                let state: &mut dyn Any = &mut *state.borrow_mut();
                data.ui.apply(object, state, ui_state, ui, ctx, graph);
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
        args: ParsedArguments,
    ) -> Result<Rc<RefCell<dyn Any>>, ()> {
        let object_type = object.get_type();
        match self.mapping.get(&object_type) {
            Some(data) => data.ui.make_ui_state(object, args),
            None => panic!(
                "Tried to create ui state for an object of unrecognized type \"{}\"",
                object_type.name()
            ),
        }
    }

    pub(crate) fn create_default_state(
        &self,
        object: &GraphObjectHandle<G::Graph>,
    ) -> Rc<RefCell<dyn Any>> {
        self.create_state_impl(object, ParsedArguments::new_empty())
            .unwrap()
    }

    pub(crate) fn create_state_from_archive(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        chive_out: ChiveOut,
    ) -> Result<Rc<RefCell<dyn Any>>, ()> {
        self.create_state_impl(object, ParsedArguments::new_empty())
            .map(|s| {
                todo!("Does anything need to be deserialized here??? Why does this even exist?")
            })
    }

    pub(crate) fn create_state_from_arguments(
        &self,
        object: &GraphObjectHandle<G::Graph>,
        args: ParsedArguments,
    ) -> Result<Rc<RefCell<dyn Any>>, ()> {
        self.create_state_impl(object, args)
    }
}
