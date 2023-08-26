use std::{any::type_name, ops::Deref, sync::Arc};

use inkwell::values::FloatValue;
use serialization::Serializer;

use crate::core::{
    graph::graphobject::{
        GraphObject, GraphObjectHandle, ObjectHandle, ObjectInitialization, ObjectType,
        WithObjectType,
    },
    jit::codegen::CodeGen,
    uniqueid::UniqueId,
};

use super::{numbergraph::NumberGraph, numbersourcetools::NumberSourceTools};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberSourceId(usize);

impl NumberSourceId {
    pub(crate) fn new(value: usize) -> NumberSourceId {
        NumberSourceId(value)
    }
}

impl Default for NumberSourceId {
    fn default() -> NumberSourceId {
        NumberSourceId(1)
    }
}

impl UniqueId for NumberSourceId {
    fn value(&self) -> usize {
        self.0
    }

    fn next(&self) -> NumberSourceId {
        NumberSourceId(self.0 + 1)
    }
}

#[derive(Copy, Clone)]
pub struct NumberConfig {
    samplewise_temporal: bool,
    sample_offset: usize,
}

impl NumberConfig {
    pub fn samplewise_temporal_at(sample_offset: usize) -> NumberConfig {
        NumberConfig {
            samplewise_temporal: true,
            sample_offset,
        }
    }

    pub fn atemporal_at(sample_offset: usize) -> NumberConfig {
        NumberConfig {
            samplewise_temporal: false,
            sample_offset,
        }
    }

    pub fn is_samplewise_temporal(&self) -> bool {
        self.samplewise_temporal
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }
}

// Intended for concrete number source types,
// hence the new() associated function
pub trait PureNumberSource: 'static + Sync + Send + WithObjectType {
    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn serialize(&self, _serializer: Serializer) {}
}

// Intended for type-erased number sources
pub trait NumberSource: 'static + Sync + Send {
    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph>;
}

pub struct NumberSourceWithId<T: PureNumberSource> {
    source: T,
    id: NumberSourceId,
}

impl<T: PureNumberSource> NumberSourceWithId<T> {
    pub(crate) fn new(source: T, id: NumberSourceId) -> NumberSourceWithId<T> {
        NumberSourceWithId { source, id }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }
}

impl<T: PureNumberSource> Deref for NumberSourceWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.source
    }
}

impl<T: PureNumberSource> NumberSource for NumberSourceWithId<T> {
    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        self.source.compile(codegen, inputs)
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self)
    }
}

impl<T: PureNumberSource> GraphObject<NumberGraph> for NumberSourceWithId<T> {
    fn create(
        graph: &mut NumberGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<NumberGraph>, ()> {
        graph
            .add_number_source::<T>(init)
            .map(|h| h.into_graph_object())
    }

    fn get_type() -> ObjectType {
        T::TYPE
    }

    fn get_dynamic_type(&self) -> ObjectType {
        T::TYPE
    }

    fn get_id(&self) -> NumberSourceId {
        self.id
    }

    fn into_arc_any(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn serialize(&self, serializer: Serializer) {
        (&*self as &T).serialize(serializer);
    }
}

pub struct NumberSourceHandle<T: PureNumberSource> {
    instance: Arc<NumberSourceWithId<T>>,
}

impl<T: PureNumberSource> NumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<NumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<NumberSourceWithId<T>>() {
            Ok(obj) => Some(NumberSourceHandle::new(obj)),
            Err(_) => None,
        }
    }

    pub fn id(&self) -> NumberSourceId {
        self.instance.id()
    }

    pub fn into_graph_object(self) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self.instance)
    }
}

impl<T: PureNumberSource> Deref for NumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: PureNumberSource> Clone for NumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: PureNumberSource> ObjectHandle<NumberGraph> for NumberSourceHandle<T> {
    type ObjectType = NumberSourceWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        NumberSourceHandle::from_graph_object(object)
    }
}
