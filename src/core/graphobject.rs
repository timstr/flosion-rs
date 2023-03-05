use std::{
    any::{type_name, Any},
    sync::Arc,
};

use super::{
    arguments::ParsedArguments,
    numberinput::NumberInputId,
    numbersource::{
        NumberSourceId, PureNumberSource, PureNumberSourceHandle, PureNumberSourceWithId,
    },
    serialization::{Deserializer, Serializable, Serializer},
    soundinput::SoundInputId,
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
    },
    uniqueid::UniqueId,
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
            GraphId::SoundInput(id) => id.value(),
            GraphId::SoundProcessor(id) => id.value(),
            GraphId::NumberInput(id) => id.value(),
            GraphId::NumberSource(id) => id.value(),
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

    pub(super) fn into_pure_number_source<T: PureNumberSource>(
        self,
    ) -> Option<PureNumberSourceHandle<T>> {
        let arc_any = self.instance.into_arc_any();
        match arc_any.downcast::<PureNumberSourceWithId<T>>() {
            Ok(obj) => Some(PureNumberSourceHandle::new(obj)),
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

impl<T: PureNumberSource> ObjectHandle for PureNumberSourceHandle<T> {
    type Type = T;

    fn from_graph_object(object: GraphObjectHandle) -> Option<Self> {
        object.into_pure_number_source()
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

impl<T: PureNumberSource> GraphObject for PureNumberSourceWithId<T> {
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
