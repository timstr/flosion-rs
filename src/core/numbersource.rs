use std::{ops::Deref, sync::Arc};

use inkwell::values::FloatValue;

use super::{
    compilednumberinput::CodeGen,
    graphobject::{ObjectInitialization, WithObjectType},
    numbersourcetools::NumberSourceTools,
    serialization::Serializer,
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberSourceId(usize);

impl NumberSourceId {
    pub(crate) fn new(id: usize) -> NumberSourceId {
        NumberSourceId(id)
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

pub(crate) enum NumberInputOwner {
    NumberSource(NumberSourceId),
    ParentGraph,
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

// TODO: now that trait SoundNumberSource is separate, consider merging this with PureNumberSource below
pub(crate) trait NumberSource: 'static + Sync + Send {
    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        _inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;
}

// TODO: repurpose this for use strictly within NumberGraphs
pub trait PureNumberSource: 'static + Sync + Send + WithObjectType {
    fn new(tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()>
    where
        Self: Sized;

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        _inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx>;

    fn serialize(&self, _serializer: Serializer) {}
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
    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        self.source.compile(codegen, inputs)
    }
}

pub struct PureNumberSourceHandle<T: PureNumberSource> {
    instance: Arc<PureNumberSourceWithId<T>>,
}

impl<T: PureNumberSource> PureNumberSourceHandle<T> {
    pub(super) fn new(instance: Arc<PureNumberSourceWithId<T>>) -> Self {
        Self { instance }
    }

    pub fn id(&self) -> NumberSourceId {
        self.instance.id()
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
