use std::{any::Any, collections::HashMap, rc::Rc};

use chive::ChiveIn;

use crate::{core::objecttype::ObjectType, ui_core::arguments::ParsedArguments};

use super::{soundgraph::SoundGraph, soundgraphid::SoundObjectId};

pub trait SoundGraphObject: Send {
    fn create(graph: &mut SoundGraph, args: &ParsedArguments) -> Result<AnySoundObjectHandle, ()>
    where
        Self: Sized;

    fn get_type() -> ObjectType
    where
        Self: Sized;

    fn get_dynamic_type(&self) -> ObjectType;

    fn get_id(&self) -> SoundObjectId;
    fn into_rc_any(self: Rc<Self>) -> Rc<dyn Any>;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, chive_in: ChiveIn);
}

// TODO: this is used exclusively for looking up processor types from handles and for
// downcasting type-erased handles. Rename it to something more suitable
pub trait SoundObjectHandle: Sized {
    // TODO: consider renaming this
    type ObjectType: SoundGraphObject;

    fn from_graph_object(object: AnySoundObjectHandle) -> Option<Self>;

    fn object_type() -> ObjectType;
}

#[derive(Clone)]
pub struct AnySoundObjectHandle {
    instance: Rc<dyn SoundGraphObject>,
}

impl AnySoundObjectHandle {
    pub(crate) fn new(instance: Rc<dyn SoundGraphObject>) -> Self {
        Self { instance }
    }

    pub(crate) fn id(&self) -> SoundObjectId {
        self.instance.get_id()
    }

    pub(crate) fn get_type(&self) -> ObjectType {
        self.instance.get_dynamic_type()
    }

    pub(crate) fn into_instance_rc(self) -> Rc<dyn SoundGraphObject> {
        self.instance
    }
}

struct SoundObjectData {
    create: Box<dyn Fn(&mut SoundGraph, &ParsedArguments) -> Result<AnySoundObjectHandle, ()>>,
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

    pub fn register<T: SoundGraphObject>(&mut self) {
        let create = |g: &mut SoundGraph,
                      args: &ParsedArguments|
         -> Result<AnySoundObjectHandle, ()> { T::create(g, args) };
        self.mapping.insert(
            T::get_type().name(),
            SoundObjectData {
                create: Box::new(create),
            },
        );
    }

    pub(crate) fn create(
        &self,
        object_type_str: &str,
        graph: &mut SoundGraph,
        args: &ParsedArguments,
    ) -> Result<AnySoundObjectHandle, ()> {
        match self.mapping.get(object_type_str) {
            Some(data) => (*data.create)(graph, args),
            None => panic!(
                "Tried to create a sound object of unrecognized type \"{}\"",
                object_type_str
            ),
        }
    }
}
