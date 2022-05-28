use std::any::{type_name, Any};

use super::{
    numberinput::NumberInputId,
    numbersource::{NumberSourceId, PureNumberSource},
    soundinput::SoundInputId,
    soundprocessor::{
        DynamicSoundProcessor, SoundProcessorId, StaticSoundProcessor,
        WrappedDynamicSoundProcessor, WrappedStaticSoundProcessor,
    },
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

impl From<NumberSourceId> for ObjectId {
    fn from(id: NumberSourceId) -> ObjectId {
        ObjectId::Number(id)
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum GraphId {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
    NumberInput(NumberInputId),
    NumberSource(NumberSourceId),
}

impl GraphId {
    pub fn inner_value(&self) -> usize {
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

pub trait GraphObject {
    fn get_type(&self) -> ObjectType;
    fn as_any(&self) -> &dyn Any;
    fn get_language_type_name(&self) -> &'static str;
}

pub trait WithObjectType: 'static {
    const TYPE: ObjectType;
}

pub trait ObjectWrapper: 'static {
    type Type: WithObjectType;

    fn get_object(&self) -> &Self::Type;
}

impl<T: DynamicSoundProcessor> GraphObject for WrappedDynamicSoundProcessor<T> {
    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }
}

impl<T: StaticSoundProcessor> GraphObject for WrappedStaticSoundProcessor<T> {
    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }
}

impl<T: PureNumberSource + WithObjectType> GraphObject for T {
    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }
}
