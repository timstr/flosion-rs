use std::any::{type_name, Any};

use super::{
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, PureNumberSource, PureNumberSourceHandle},
    serialization::{Deserializer, Serializable, Serializer},
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

impl Serializable for ObjectId {
    fn serialize(&self, serializer: &mut Serializer) {
        match self {
            ObjectId::Sound(spid) => {
                serializer.u8(1);
                serializer.u32(spid.0 as u32);
            }
            ObjectId::Number(nsid) => {
                serializer.u8(2);
                serializer.u32(nsid.0 as u32);
            }
        }
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        match deserializer.u8()? {
            1 => Ok(ObjectId::Sound(SoundProcessorId(
                deserializer.u32()? as usize
            ))),
            2 => Ok(ObjectId::Number(NumberSourceId(
                deserializer.u32()? as usize
            ))),
            _ => Err(()),
        }
    }
}

impl From<SoundProcessorId> for ObjectId {
    fn from(id: SoundProcessorId) -> ObjectId {
        ObjectId::Sound(id)
    }
}

impl From<NumberSourceId> for ObjectId {
    fn from(id: NumberSourceId) -> ObjectId {
        ObjectId::Number(id)
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

impl From<&SoundInputId> for GraphId {
    fn from(id: &SoundInputId) -> GraphId {
        GraphId::SoundInput(*id)
    }
}
impl From<&SoundProcessorId> for GraphId {
    fn from(id: &SoundProcessorId) -> GraphId {
        GraphId::SoundProcessor(*id)
    }
}
impl From<&NumberInputId> for GraphId {
    fn from(id: &NumberInputId) -> GraphId {
        GraphId::NumberInput(*id)
    }
}
impl From<&NumberSourceId> for GraphId {
    fn from(id: &NumberSourceId) -> GraphId {
        GraphId::NumberSource(*id)
    }
}

pub trait GraphObject {
    fn get_type(&self) -> ObjectType;
    fn as_any(&self) -> &dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, _serializer: Serializer);
}

pub trait TypedGraphObject: GraphObject {
    type Type;
}

impl<T: SoundProcessor> TypedGraphObject for SoundProcessorHandle<T> {
    type Type = T;
}

impl<T: SoundProcessor> GraphObject for SoundProcessorHandle<T> {
    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, mut serializer: Serializer) {
        serializer.object(&ObjectId::Sound(self.id()));
        serializer.string(T::TYPE.name());
        let s = serializer.subarchive();
        self.instance().serialize(s);
    }
}

impl<T: PureNumberSource> TypedGraphObject for PureNumberSourceHandle<T> {
    type Type = T;
}

impl<T: PureNumberSource> GraphObject for PureNumberSourceHandle<T> {
    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, _serializer: Serializer) {
        todo!()
    }
}

pub trait WithObjectType: 'static {
    const TYPE: ObjectType;
}

// pub trait ObjectWrapper: 'static {
//     type Type: WithObjectType;

//     fn get_object(&self) -> &Self::Type;
// }

// impl<T: SoundProcessor> ObjectWrapper for SoundProcessorHandle<T> {
//     type Type = T;

//     fn get_object(&self) -> &T {
//         self.instance()
//     }
// }

// impl<T: PureNumberSource> ObjectWrapper for PureNumberSourceHandle<T> {
//     type Type = T;

//     fn get_object(&self) -> &T {
//         self.instance()
//     }
// }

// impl<T: PureNumberSource + WithObjectType> GraphObject for T {
//     fn get_type(&self) -> ObjectType {
//         T::TYPE
//     }

//     fn as_any(&self) -> &dyn Any {
//         self
//     }

//     fn get_language_type_name(&self) -> &'static str {
//         type_name::<T>()
//     }

//     fn serialize(&self, serializer: Serializer) {
//         self.serialize(serializer);
//     }
// }
