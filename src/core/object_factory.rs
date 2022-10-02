use std::collections::HashMap;

use crate::core::{
    arguments::ParsedArguments, graphobject::ObjectInitialization, numbersource::PureNumberSource,
    serialization::Deserializer, soundprocessor::SoundProcessor,
};

use super::{
    graphobject::GraphObject, numbersource::NumberSource, soundgraphtopology::SoundGraphTopology,
    soundprocessor::SoundProcessorWrapper,
};

struct ObjectData {
    create: Box<
        dyn Fn(&mut SoundGraphTopology, ObjectInitialization) -> Result<Box<dyn GraphObject>, ()>,
    >,
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

    pub fn register_sound_processor<T: SoundProcessor>(&mut self) {
        let create = |g: &mut SoundGraphTopology,
                      init: ObjectInitialization|
         -> Result<Box<dyn GraphObject>, ()> {
            let h = g.add_sound_processor::<T>(init)?;
            Ok(h.instance_arc().as_graph_object(h.id()))
        };
        self.mapping.insert(
            T::TYPE.name(),
            ObjectData {
                create: Box::new(create),
            },
        );
    }

    pub fn register_number_source<T: PureNumberSource>(&mut self) {
        let create = |g: &mut SoundGraphTopology, init: ObjectInitialization| {
            let h = g.add_pure_number_source::<T>(init)?;
            Ok(h.instance_arc().as_graph_object(h.id()).unwrap())
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
        graph_topo: &mut SoundGraphTopology,
        init: ObjectInitialization,
    ) -> Result<Box<dyn GraphObject>, ()> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph_topo, init),
            None => panic!(
                "Tried to create an object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }

    pub fn create_from_args(
        &self,
        object_type_str: &str,
        graph_topo: &mut SoundGraphTopology,
        args: &ParsedArguments,
    ) -> Result<Box<dyn GraphObject>, ()> {
        self.create_impl(
            object_type_str,
            graph_topo,
            ObjectInitialization::Args(args),
        )
    }

    pub fn create_from_archive(
        &self,
        object_type_str: &str,
        graph_topo: &mut SoundGraphTopology,
        deserializer: Deserializer,
    ) -> Result<Box<dyn GraphObject>, ()> {
        self.create_impl(
            object_type_str,
            graph_topo,
            ObjectInitialization::Archive(deserializer),
        )
    }
}
