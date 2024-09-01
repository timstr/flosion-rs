use std::collections::HashMap;

use chive::ChiveOut;

use crate::ui_core::arguments::ParsedArguments;

use super::{
    graph::Graph,
    graphobject::{GraphObject, GraphObjectHandle},
};

struct ObjectData<G: Graph> {
    create: Box<dyn Fn(&mut G, ParsedArguments) -> Result<GraphObjectHandle<G>, ()>>,
}

pub struct ObjectFactory<G: Graph> {
    mapping: HashMap<&'static str, ObjectData<G>>,
}

impl<G: Graph> ObjectFactory<G> {
    pub fn new_empty() -> ObjectFactory<G> {
        ObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: GraphObject<G>>(&mut self) {
        let create = |g: &mut G, args: ParsedArguments| -> Result<GraphObjectHandle<G>, ()> {
            T::create(g, args)
        };
        self.mapping.insert(
            T::get_type().name(),
            ObjectData {
                create: Box::new(create),
            },
        );
    }

    fn create_impl(
        &self,
        object_type_str: &str,
        graph: &mut G,
        args: ParsedArguments,
    ) -> Result<GraphObjectHandle<G>, ()> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph, args),
            None => panic!(
                "Tried to create an object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }

    // TODO: is this needed?
    pub(crate) fn create_default(
        &self,
        object_type_str: &str,
        graph: &mut G,
    ) -> Result<GraphObjectHandle<G>, ()> {
        self.create_impl(object_type_str, graph, ParsedArguments::new_empty())
    }

    pub(crate) fn create_from_args(
        &self,
        object_type_str: &str,
        graph: &mut G,
        args: ParsedArguments,
    ) -> Result<GraphObjectHandle<G>, ()> {
        self.create_impl(object_type_str, graph, args)
    }

    // TODO: is this needed?
    pub(crate) fn create_from_archive(
        &self,
        object_type_str: &str,
        graph: &mut G,
        chive_out: ChiveOut,
    ) -> Result<GraphObjectHandle<G>, ()> {
        self.create_impl(object_type_str, graph, ParsedArguments::new_empty())
            .map(|hande| {
                todo!("Restore the state of the object from the Chive using e.g. rollback()")
            })
    }
}
