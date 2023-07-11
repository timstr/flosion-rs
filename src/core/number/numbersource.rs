use std::{ops::Deref, sync::Arc};

use inkwell::values::FloatValue;

use crate::core::{
    graph::graphobject::ObjectInitialization, jit::codegen::CodeGen, serialization::Serializer,
    uniqueid::UniqueId,
};

use super::numbersourcetools::NumberSourceTools;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberSourceId(usize);

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
pub trait PureNumberSource: 'static + Sync + Send {
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
}

pub struct NumberSourceHandle<T: PureNumberSource> {
    instance: Arc<NumberSourceWithId<T>>,
}

impl<T: PureNumberSource> NumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<NumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub fn id(&self) -> NumberSourceId {
        self.instance.id()
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
