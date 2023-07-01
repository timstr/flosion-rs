use std::collections::HashMap;

use crate::core::{arguments::ParsedArguments, serialization::Deserializer};

use super::sound::{
    graphobject::{GraphObjectHandle, ObjectInitialization},
    soundgraph::SoundGraph,
    soundprocessor::{DynamicSoundProcessor, StaticSoundProcessor},
};

struct ObjectData {
    create: Box<dyn Fn(&mut SoundGraph, ObjectInitialization) -> Result<GraphObjectHandle, ()>>,
}

pub struct ObjectFactory {
    mapping: HashMap<&'static str, ObjectData>,
}

impl ObjectFactory {
    pub fn new_empty() -> ObjectFactory {
        ObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register_static_sound_processor<T: StaticSoundProcessor>(&mut self) {
        let create =
            |g: &mut SoundGraph, init: ObjectInitialization| -> Result<GraphObjectHandle, ()> {
                let h = g.add_static_sound_processor::<T>(init)?;
                Ok(h.into_graph_object())
            };
        self.mapping.insert(
            T::TYPE.name(),
            ObjectData {
                create: Box::new(create),
            },
        );
    }

    pub fn register_dynamic_sound_processor<T: DynamicSoundProcessor>(&mut self) {
        let create =
            |g: &mut SoundGraph, init: ObjectInitialization| -> Result<GraphObjectHandle, ()> {
                let h = g.add_dynamic_sound_processor::<T>(init)?;
                Ok(h.into_graph_object())
            };
        self.mapping.insert(
            T::TYPE.name(),
            ObjectData {
                create: Box::new(create),
            },
        );
    }

    fn create_impl(
        &self,
        object_type_str: &str,
        graph: &mut SoundGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle, ()> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph, init),
            None => panic!(
                "Tried to create an object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }

    pub(crate) fn create_from_args(
        &self,
        object_type_str: &str,
        graph: &mut SoundGraph,
        args: &ParsedArguments,
    ) -> Result<GraphObjectHandle, ()> {
        self.create_impl(object_type_str, graph, ObjectInitialization::Args(args))
    }

    pub(crate) fn create_from_archive(
        &self,
        object_type_str: &str,
        graph: &mut SoundGraph,
        deserializer: Deserializer,
    ) -> Result<GraphObjectHandle, ()> {
        self.create_impl(
            object_type_str,
            graph,
            ObjectInitialization::Archive(deserializer),
        )
    }
}
