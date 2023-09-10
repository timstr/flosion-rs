use std::collections::HashMap;

use serialization::Deserializer;

use super::{
    graph::Graph,
    graphobject::{GraphObject, GraphObjectHandle, ObjectInitialization},
};

struct ObjectData<G: Graph> {
    create: Box<dyn Fn(&mut G, ObjectInitialization) -> Result<GraphObjectHandle<G>, ()>>,
}

// TODO: share this between number and sound graphs,
// consider adding a generic parameter for the graph type
// (SoundGraph vs NumberGraph) and putting the creation
// code into a trait UHNNNNG but how to avoid issues with
// theoretically possible multiple conflicting trait implementations?

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
        let create = |g: &mut G, init: ObjectInitialization| -> Result<GraphObjectHandle<G>, ()> {
            T::create(g, init)
        };
        self.mapping.insert(
            T::get_type().name(),
            ObjectData {
                create: Box::new(create),
            },
        );
    }

    // pub fn register_static_sound_processor<T: StaticSoundProcessor>(&mut self) {
    //     // TODO: move this to a trait
    //     let create =
    //         |g: &mut SoundGraph, init: ObjectInitialization| -> Result<GraphObjectHandle, ()> {
    //             let h = g.add_static_sound_processor::<T>(init)?;
    //             Ok(h.into_graph_object())
    //         };
    //     self.mapping.insert(
    //         T::TYPE.name(),
    //         ObjectData {
    //             create: Box::new(create),
    //         },
    //     );
    // }

    // pub fn register_dynamic_sound_processor<T: DynamicSoundProcessor>(&mut self) {
    //     // TODO: move this to a trait
    //     let create =
    //         |g: &mut SoundGraph, init: ObjectInitialization| -> Result<GraphObjectHandle, ()> {
    //             let h = g.add_dynamic_sound_processor::<T>(init)?;
    //             Ok(h.into_graph_object())
    //         };
    //     self.mapping.insert(
    //         T::TYPE.name(),
    //         ObjectData {
    //             create: Box::new(create),
    //         },
    //     );
    // }

    fn create_impl(
        &self,
        object_type_str: &str,
        graph: &mut G,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<G>, ()> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph, init),
            None => panic!(
                "Tried to create an object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }

    pub(crate) fn create_default(
        &self,
        object_type_str: &str,
        graph: &mut G,
    ) -> Result<GraphObjectHandle<G>, ()> {
        self.create_impl(object_type_str, graph, ObjectInitialization::Default)
    }

    pub(crate) fn create_from_archive(
        &self,
        object_type_str: &str,
        graph: &mut G,
        deserializer: Deserializer,
    ) -> Result<GraphObjectHandle<G>, ()> {
        self.create_impl(
            object_type_str,
            graph,
            ObjectInitialization::Archive(deserializer),
        )
    }
}
