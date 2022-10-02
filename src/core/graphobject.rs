use std::any::{type_name, Any};

use super::{
    arguments::ParsedArguments,
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, PureNumberSource, PureNumberSourceHandle},
    serialization::{Deserializer, Serializer},
    soundinput::SoundInputId,
    soundprocessor::{SoundProcessor, SoundProcessorHandle, SoundProcessorId},
};

#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ObjectType {
    name: &'static str,
}

impl ObjectType {
    pub const fn new(name: &'static str) -> ObjectType {
        ObjectType { name }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Hash)]
pub enum ObjectId {
    Sound(SoundProcessorId),
    Number(NumberSourceId),
}

impl ObjectId {
    pub fn as_sound_processor_id(&self) -> Option<SoundProcessorId> {
        match self {
            ObjectId::Sound(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_number_source_id(&self) -> Option<NumberSourceId> {
        match self {
            ObjectId::Number(id) => Some(*id),
            _ => None,
        }
    }
}

impl From<SoundProcessorId> for ObjectId {
    fn from(id: SoundProcessorId) -> ObjectId {
        ObjectId::Sound(id)
    }
}
impl From<&SoundProcessorId> for ObjectId {
    fn from(id: &SoundProcessorId) -> ObjectId {
        ObjectId::Sound(*id)
    }
}

impl From<NumberSourceId> for ObjectId {
    fn from(id: NumberSourceId) -> ObjectId {
        ObjectId::Number(id)
    }
}
impl From<&NumberSourceId> for ObjectId {
    fn from(id: &NumberSourceId) -> ObjectId {
        ObjectId::Number(*id)
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub enum GraphId {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
    NumberInput(NumberInputId),
    NumberSource(NumberSourceId),
}

impl GraphId {
    pub fn as_usize(&self) -> usize {
        match self {
            GraphId::SoundInput(id) => id.0,
            GraphId::SoundProcessor(id) => id.0,
            GraphId::NumberInput(id) => id.0,
            GraphId::NumberSource(id) => id.0,
        }
    }
}

impl From<SoundInputId> for GraphId {
    fn from(id: SoundInputId) -> GraphId {
        GraphId::SoundInput(id)
    }
}
impl From<SoundProcessorId> for GraphId {
    fn from(id: SoundProcessorId) -> GraphId {
        GraphId::SoundProcessor(id)
    }
}
impl From<NumberInputId> for GraphId {
    fn from(id: NumberInputId) -> GraphId {
        GraphId::NumberInput(id)
    }
}
impl From<NumberSourceId> for GraphId {
    fn from(id: NumberSourceId) -> GraphId {
        GraphId::NumberSource(id)
    }
}
impl From<ObjectId> for GraphId {
    fn from(id: ObjectId) -> GraphId {
        match id {
            ObjectId::Sound(i) => GraphId::SoundProcessor(i),
            ObjectId::Number(i) => GraphId::NumberSource(i),
        }
    }
}

pub trait GraphObject {
    fn get_id(&self) -> ObjectId;
    fn get_type(&self) -> ObjectType;
    fn as_any(&self) -> &dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, serializer: Serializer);
}

pub fn object_to_sound_processor<T: SoundProcessor>(
    object: &dyn GraphObject,
) -> Option<SoundProcessorHandle<T>> {
    let h = object.as_any().downcast_ref::<SoundProcessorHandle<T>>();
    h.map(|h| h.clone())
}

pub fn object_to_number_source<T: PureNumberSource>(
    object: &dyn GraphObject,
) -> Option<PureNumberSourceHandle<T>> {
    let h = object.as_any().downcast_ref::<PureNumberSourceHandle<T>>();
    h.map(|h| h.clone())
}

pub trait TypedGraphObject: GraphObject {
    type Type;
}

impl<T: SoundProcessor> TypedGraphObject for SoundProcessorHandle<T> {
    type Type = T;
}

impl<T: SoundProcessor> GraphObject for SoundProcessorHandle<T> {
    fn get_id(&self) -> ObjectId {
        self.id().into()
    }

    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, serializer: Serializer) {
        self.instance().serialize(serializer);
    }
}

impl<T: PureNumberSource> TypedGraphObject for PureNumberSourceHandle<T> {
    type Type = T;
}

impl<T: PureNumberSource> GraphObject for PureNumberSourceHandle<T> {
    fn get_id(&self) -> ObjectId {
        self.id().into()
    }

    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, serializer: Serializer) {
        self.instance().serialize(serializer);
    }
}

pub enum ObjectInitialization<'a> {
    Args(&'a ParsedArguments),
    Archive(Deserializer<'a>),
    Default,
}

pub trait WithObjectType: 'static {
    const TYPE: ObjectType;
}
