use std::sync::Arc;

use super::{
    context::NumberContext,
    graphobject::{ObjectWrapper, WithObjectType},
    key::Key,
    numbersourcetools::NumberSourceTools,
    soundinput::{KeyedSoundInputHandle, SoundInputId},
    soundprocessor::{SoundProcessorData, SoundProcessorId},
    soundstate::{SoundState, StateOwner},
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NumberSourceId(pub usize);

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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NumberSourceOwner {
    Nothing,
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
}

impl NumberSourceOwner {
    pub fn is_stateful(&self) -> bool {
        match self {
            NumberSourceOwner::Nothing => false,
            NumberSourceOwner::SoundProcessor(_) => true,
            NumberSourceOwner::SoundInput(_) => true,
        }
    }

    pub fn as_state_owner(&self) -> Option<StateOwner> {
        match self {
            NumberSourceOwner::Nothing => None,
            NumberSourceOwner::SoundProcessor(spid) => Some(StateOwner::SoundProcessor(*spid)),
            NumberSourceOwner::SoundInput(siid) => Some(StateOwner::SoundInput(*siid)),
        }
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

pub trait NumberSource: 'static + Sync + Send {
    fn eval(&self, dst: &mut [f32], context: NumberContext);
}

pub trait PureNumberSource: NumberSource + WithObjectType {
    fn new(tools: &mut NumberSourceTools<'_>) -> Self
    where
        Self: Sized;
}

pub struct PureNumberSourceHandle<T: PureNumberSource> {
    id: NumberSourceId,
    instance: Arc<T>,
}

impl<T: PureNumberSource> PureNumberSourceHandle<T> {
    pub(super) fn new(id: NumberSourceId, instance: Arc<T>) -> PureNumberSourceHandle<T> {
        PureNumberSourceHandle { id, instance }
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }

    pub fn instance(&self) -> &T {
        &*self.instance
    }
}

impl<T: PureNumberSource> ObjectWrapper for T {
    type Type = T;

    fn get_object(&self) -> &T {
        &self
    }
}

pub trait StateFunction<T: SoundState>: 'static + Sized + Sync + Send {
    fn apply(&self, dst: &mut [f32], state: &T);
}

impl<T: SoundState, F: 'static + Sized + Sync + Send> StateFunction<T> for F
where
    F: Fn(&mut [f32], &T),
{
    fn apply(&self, dst: &mut [f32], state: &T) {
        (*self)(dst, state);
    }
}

pub struct ProcessorNumberSource<T: SoundState, F: StateFunction<T>> {
    data: Arc<SoundProcessorData<T>>,
    function: F,
}

impl<T: SoundState, F: StateFunction<T>> ProcessorNumberSource<T, F> {
    pub(super) fn new(
        data: Arc<SoundProcessorData<T>>,
        function: F,
    ) -> ProcessorNumberSource<T, F> {
        ProcessorNumberSource { data, function }
    }
}

impl<T: SoundState, F: StateFunction<T>> NumberSource for ProcessorNumberSource<T, F> {
    fn eval(&self, dst: &mut [f32], context: NumberContext) {
        let state = context.sound_processor_state(&self.data);
        self.function.apply(dst, &state.read());
    }
}

pub struct NumberSourceHandle {
    id: NumberSourceId,
    owner: NumberSourceOwner,
}

impl NumberSourceHandle {
    pub(super) fn new(id: NumberSourceId, owner: NumberSourceOwner) -> NumberSourceHandle {
        NumberSourceHandle { id, owner }
    }

    pub fn id(&self) -> NumberSourceId {
        self.id
    }

    pub fn owner(&self) -> NumberSourceOwner {
        self.owner
    }
}

// pub struct SingleInputNumberSource<F: StateFunction<EmptyState>> {
//     handle: SingleSoundInputHandle,
//     function: F,
// }

// impl<F: StateFunction<EmptyState>> SingleInputNumberSource<F> {
//     pub(super) fn new(handle: SingleSoundInputHandle, function: F) -> SingleInputNumberSource<F> {
//         SingleInputNumberSource { handle, function }
//     }
// }

// impl<F: StateFunction<EmptyState>> NumberSource for SingleInputNumberSource<F> {
//     fn eval(&self, dst: &mut [f32], context: NumberContext) {
//         let state = context.single_input_state(&self.handle);
//         self.function.apply(dst, &state.read());
//     }
// }

// TODO: elaborate to allow the current key to be passed by reference to the function in addition to the state
pub struct KeyedInputNumberSource<K: Key, T: SoundState, F: StateFunction<T>> {
    handle: KeyedSoundInputHandle<K, T>,
    function: F,
}

impl<K: Key, T: SoundState, F: StateFunction<T>> KeyedInputNumberSource<K, T, F> {
    pub(super) fn new(
        handle: KeyedSoundInputHandle<K, T>,
        function: F,
    ) -> KeyedInputNumberSource<K, T, F> {
        KeyedInputNumberSource { handle, function }
    }
}

impl<K: Key, T: SoundState, F: StateFunction<T>> NumberSource for KeyedInputNumberSource<K, T, F> {
    fn eval(&self, dst: &mut [f32], context: NumberContext) {
        let state = context.keyed_input_state(&self.handle);
        self.function.apply(dst, &state.read().state());
    }
}
