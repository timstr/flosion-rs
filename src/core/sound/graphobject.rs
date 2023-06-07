use std::{
    any::{type_name, Any},
    sync::Arc,
};

use crate::core::{
    arguments::ParsedArguments,
    serialization::{Deserializer, Serializer},
    uniqueid::UniqueId,
};

use super::{
    soundinput::SoundInputId,
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId,
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
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

// Used to refer to top-level objects in the sound graph,
// e.g. sound processors (TODO and number function definitions)
// that have free-floating representations
// TODO: rename to TopLevelSoundGraphId or something shorter but just as meaningful
#[derive(Eq, PartialEq, Clone, Copy, Hash)]
pub enum ObjectId {
    Sound(SoundProcessorId),
}

impl ObjectId {
    pub fn as_sound_processor_id(&self) -> Option<SoundProcessorId> {
        match self {
            ObjectId::Sound(id) => Some(*id),
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

#[derive(Eq, PartialEq, Clone, Copy, Debug, Hash)]
pub enum SoundGraphId {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
    SoundNumberInput(SoundNumberInputId),
    SoundNumberSource(SoundNumberSourceId),
}

impl SoundGraphId {
    pub fn as_usize(&self) -> usize {
        match self {
            SoundGraphId::SoundInput(id) => id.value(),
            SoundGraphId::SoundProcessor(id) => id.value(),
            SoundGraphId::SoundNumberInput(id) => id.value(),
            SoundGraphId::SoundNumberSource(id) => id.value(),
        }
    }
}

impl From<SoundInputId> for SoundGraphId {
    fn from(id: SoundInputId) -> SoundGraphId {
        SoundGraphId::SoundInput(id)
    }
}
impl From<SoundProcessorId> for SoundGraphId {
    fn from(id: SoundProcessorId) -> SoundGraphId {
        SoundGraphId::SoundProcessor(id)
    }
}
impl From<SoundNumberInputId> for SoundGraphId {
    fn from(id: SoundNumberInputId) -> SoundGraphId {
        SoundGraphId::SoundNumberInput(id)
    }
}
impl From<SoundNumberSourceId> for SoundGraphId {
    fn from(id: SoundNumberSourceId) -> SoundGraphId {
        SoundGraphId::SoundNumberSource(id)
    }
}
impl From<ObjectId> for SoundGraphId {
    fn from(id: ObjectId) -> SoundGraphId {
        match id {
            ObjectId::Sound(i) => SoundGraphId::SoundProcessor(i),
        }
    }
}

pub trait GraphObject: 'static + Send + Sync {
    fn get_id(&self) -> ObjectId;
    fn get_type(&self) -> ObjectType;
    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, serializer: Serializer);
}

#[derive(Clone)]
pub struct GraphObjectHandle {
    instance: Arc<dyn GraphObject>,
}

impl GraphObjectHandle {
    pub(super) fn new(instance: Arc<dyn GraphObject>) -> Self {
        Self { instance }
    }

    pub(crate) fn id(&self) -> ObjectId {
        self.instance.get_id()
    }

    pub(crate) fn get_type(&self) -> ObjectType {
        self.instance.get_type()
    }

    pub(crate) fn instance(&self) -> &dyn GraphObject {
        &*self.instance
    }

    pub(super) fn into_static_sound_processor<T: StaticSoundProcessor>(
        self,
    ) -> Option<StaticSoundProcessorHandle<T>> {
        let arc_any = self.instance.into_arc_any();
        match arc_any.downcast::<StaticSoundProcessorWithId<T>>() {
            Ok(obj) => Some(StaticSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub(super) fn into_dynamic_sound_processor<T: DynamicSoundProcessor>(
        self,
    ) -> Option<DynamicSoundProcessorHandle<T>> {
        let arc_any = self.instance.into_arc_any();
        match arc_any.downcast::<DynamicSoundProcessorWithId<T>>() {
            Ok(obj) => Some(DynamicSoundProcessorHandle::new(obj)),
            Err(_) => None,
        }
    }
}

pub trait ObjectHandle: Sized {
    type Type;

    fn from_graph_object(object: GraphObjectHandle) -> Option<Self>;
}

impl<T: StaticSoundProcessor> ObjectHandle for StaticSoundProcessorHandle<T> {
    type Type = T;

    fn from_graph_object(object: GraphObjectHandle) -> Option<Self> {
        object.into_static_sound_processor()
    }
}

impl<T: DynamicSoundProcessor> ObjectHandle for DynamicSoundProcessorHandle<T> {
    type Type = T;

    fn from_graph_object(object: GraphObjectHandle) -> Option<Self> {
        object.into_dynamic_sound_processor()
    }
}

impl<T: StaticSoundProcessor> GraphObject for StaticSoundProcessorWithId<T> {
    fn get_id(&self) -> ObjectId {
        self.id().into()
    }

    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        (&*self as &T).serialize(serializer);
    }
}

impl<T: DynamicSoundProcessor> GraphObject for DynamicSoundProcessorWithId<T> {
    fn get_id(&self) -> ObjectId {
        self.id().into()
    }

    fn get_type(&self) -> ObjectType {
        T::TYPE
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        let s: &T = &*self;
        s.serialize(serializer);
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
