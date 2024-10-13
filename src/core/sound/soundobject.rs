use std::{any::Any, collections::HashMap};

use crate::{core::objecttype::ObjectType, ui_core::arguments::ParsedArguments};

use super::{soundgraph::SoundGraph, soundgraphid::SoundObjectId};

pub trait SoundGraphObject {
    fn create<'a>(graph: &'a mut SoundGraph, args: &ParsedArguments) -> &'a mut Self
    where
        Self: Sized;

    fn id(&self) -> SoundObjectId;

    fn get_type() -> ObjectType
    where
        Self: Sized;

    fn get_dynamic_type(&self) -> ObjectType;

    fn friendly_name(&self) -> String;

    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    // TODO: remove, this was only made to debug Any downcasts
    fn get_language_type_name(&self) -> &'static str;
}

struct SoundObjectData {
    create:
        Box<dyn for<'a> Fn(&'a mut SoundGraph, &ParsedArguments) -> &'a mut dyn SoundGraphObject>,
}

pub struct SoundObjectFactory {
    mapping: HashMap<&'static str, SoundObjectData>,
}

impl SoundObjectFactory {
    pub fn new_empty() -> SoundObjectFactory {
        SoundObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: 'static + SoundGraphObject>(&mut self) {
        fn create<'a, T2: 'static + SoundGraphObject>(
            g: &'a mut SoundGraph,
            args: &ParsedArguments,
        ) -> &'a mut dyn SoundGraphObject {
            T2::create(g, args)
        }

        self.mapping.insert(
            T::get_type().name(),
            SoundObjectData {
                create: Box::new(create::<T>),
            },
        );
    }

    pub(crate) fn create<'a>(
        &self,
        object_type_str: &str,
        graph: &'a mut SoundGraph,
        args: &ParsedArguments,
    ) -> &'a mut dyn SoundGraphObject {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph, args),
            None => panic!(
                "Tried to create a sound object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }
}
