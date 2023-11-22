use std::{any::type_name, ops::Deref, sync::Arc};

use inkwell::values::{FloatValue, PointerValue};
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
    fn num_variables(&self) -> usize;

    fn compile_init<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>>;

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph>;
}

pub struct PureNumberSourceWithId<T: PureNumberSource> {
    source: T,
    id: NumberSourceId,
}

impl<T: PureNumberSource> PureNumberSourceWithId<T> {
    pub(crate) fn new(source: T, id: NumberSourceId) -> PureNumberSourceWithId<T> {
        PureNumberSourceWithId { source, id }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }
}

impl<T: PureNumberSource> Deref for PureNumberSourceWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.source
    }
}

impl<T: PureNumberSource> NumberSource for PureNumberSourceWithId<T> {
    fn num_variables(&self) -> usize {
        0
    }

    fn compile_init<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
        Vec::new()
    }

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(variables.len(), 0);
        self.source.compile(codegen, inputs)
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self)
    }
}

impl<T: PureNumberSource> GraphObject<NumberGraph> for PureNumberSourceWithId<T> {
    fn create(
        graph: &mut NumberGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<NumberGraph>, ()> {
        graph
            .add_pure_number_source::<T>(init)
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

pub struct PureNumberSourceHandle<T: PureNumberSource> {
    instance: Arc<PureNumberSourceWithId<T>>,
}

impl<T: PureNumberSource> PureNumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<PureNumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<PureNumberSourceWithId<T>>() {
            Ok(obj) => Some(PureNumberSourceHandle::new(obj)),
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

impl<T: PureNumberSource> Deref for PureNumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: PureNumberSource> Clone for PureNumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: PureNumberSource> ObjectHandle<NumberGraph> for PureNumberSourceHandle<T> {
    type ObjectType = PureNumberSourceWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        PureNumberSourceHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}

pub trait StatefulNumberSource: 'static + Sync + Send + WithObjectType {
    const NUM_VARIABLES: usize;

    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn compile_init<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>>;

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn serialize(&self, _serializer: Serializer) {}
}

pub struct StatefulNumberSourceWithId<T: StatefulNumberSource> {
    source: T,
    id: NumberSourceId,
}

impl<T: StatefulNumberSource> StatefulNumberSourceWithId<T> {
    pub(crate) fn new(source: T, id: NumberSourceId) -> StatefulNumberSourceWithId<T> {
        StatefulNumberSourceWithId { source, id }
    }

    pub(crate) fn id(&self) -> NumberSourceId {
        self.id
    }
}

impl<T: StatefulNumberSource> Deref for StatefulNumberSourceWithId<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.source
    }
}

impl<T: StatefulNumberSource> NumberSource for StatefulNumberSourceWithId<T> {
    fn num_variables(&self) -> usize {
        T::NUM_VARIABLES
    }

    fn compile_init<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
        self.source.compile_init(codegen)
    }

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(variables.len(), self.num_variables());
        self.source.compile_loop(codegen, inputs, variables)
    }

    fn as_graph_object(self: Arc<Self>) -> GraphObjectHandle<NumberGraph> {
        GraphObjectHandle::new(self)
    }
}

impl<T: StatefulNumberSource> GraphObject<NumberGraph> for StatefulNumberSourceWithId<T> {
    fn create(
        graph: &mut NumberGraph,
        init: ObjectInitialization,
    ) -> Result<GraphObjectHandle<NumberGraph>, ()> {
        graph
            .add_stateful_number_source::<T>(init)
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

pub struct StatefulNumberSourceHandle<T: StatefulNumberSource> {
    instance: Arc<StatefulNumberSourceWithId<T>>,
}

impl<T: StatefulNumberSource> StatefulNumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<StatefulNumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub(super) fn from_graph_object(handle: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        let arc_any = handle.into_instance_arc().into_arc_any();
        match arc_any.downcast::<StatefulNumberSourceWithId<T>>() {
            Ok(obj) => Some(StatefulNumberSourceHandle::new(obj)),
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
impl<T: StatefulNumberSource> Deref for StatefulNumberSourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.instance
    }
}

impl<T: StatefulNumberSource> Clone for StatefulNumberSourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            instance: Arc::clone(&self.instance),
        }
    }
}

impl<T: StatefulNumberSource> ObjectHandle<NumberGraph> for StatefulNumberSourceHandle<T> {
    type ObjectType = StatefulNumberSourceWithId<T>;

    fn from_graph_object(object: GraphObjectHandle<NumberGraph>) -> Option<Self> {
        StatefulNumberSourceHandle::from_graph_object(object)
    }

    fn object_type() -> ObjectType {
        T::TYPE
    }
}
