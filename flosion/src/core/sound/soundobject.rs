use std::{any::Any, collections::HashMap};

use crate::{core::objecttype::ObjectType, ui_core::arguments::ParsedArguments};

use super::{soundgraphid::SoundObjectId, soundprocessor::AnySoundProcessor};

pub trait SoundGraphObject {
    fn create<'a>(args: &ParsedArguments) -> Self
    where
        Self: Sized;

    fn id(&self) -> SoundObjectId;

    fn get_type() -> ObjectType
    where
        Self: Sized;

    fn get_dynamic_type(&self) -> ObjectType;

    fn friendly_name(&self) -> String;

    fn as_sound_processor(&self) -> Option<&dyn AnySoundProcessor>;
    fn into_boxed_sound_processor(self: Box<Self>) -> Option<Box<dyn AnySoundProcessor>>;

    // Are these needed?
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;

    // TODO: remove, this was only made to debug Any downcasts
    fn get_language_type_name(&self) -> &'static str;
}

struct SoundObjectCreator {
    create: Box<dyn Fn(&ParsedArguments) -> Box<dyn SoundGraphObject>>,
}

pub struct SoundObjectFactory {
    mapping: HashMap<&'static str, SoundObjectCreator>,
}

impl SoundObjectFactory {
    pub fn new_empty() -> SoundObjectFactory {
        SoundObjectFactory {
            mapping: HashMap::new(),
        }
    }

    pub fn register<T: 'static + SoundGraphObject>(&mut self) {
        self.mapping.insert(
            T::get_type().name(),
            SoundObjectCreator {
                create: Box::new(|args| Box::new(T::create(args))),
            },
        );
    }

    pub(crate) fn create(
        &self,
        object_type_str: &str,
        args: &ParsedArguments,
    ) -> Box<dyn SoundGraphObject> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(args),
            None => panic!(
                "Tried to create a sound object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }
}
