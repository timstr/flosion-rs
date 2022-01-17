use std::sync::Arc;

use super::{
    context::Context,
    key::Key,
    soundinput::{KeyedSoundInputHandle, SingleSoundInputHandle, SoundInputId},
    soundprocessor::{DynamicSoundProcessorData, SoundProcessorId, StaticSoundProcessorData},
    soundstate::{EmptyState, SoundState},
    uniqueid::UniqueId,
};

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

#[derive(Debug)]
pub enum NumberSourceOwner {
    Nothing,
    SoundProcessor(SoundProcessorId),
    SoundInput(SoundInputId),
}

pub trait NumberSource: Sync + Send {
    fn eval(&self, dst: &mut [f32], context: Context);
}

pub struct NumberSourceHandle {
    id: NumberSourceId,
    instance: Arc<dyn NumberSource>,
}

impl NumberSourceHandle {
    pub(super) fn new(id: NumberSourceId, instance: Box<dyn NumberSource>) -> NumberSourceHandle {
        NumberSourceHandle {
            id,
            instance: instance.into(),
        }
    }

    pub fn instance(&self) -> &dyn NumberSource {
        &*self.instance
    }
}

impl Clone for NumberSourceHandle {
    fn clone(&self) -> NumberSourceHandle {
        NumberSourceHandle {
            id: self.id,
            instance: Arc::clone(&self.instance),
        }
    }
}

pub trait StateFunction<T: SoundState>: Sync + Send {
    fn apply(&self, dst: &mut [f32], context: Context, state: &T);
}

impl<T: SoundState, F: Sync + Send> StateFunction<T> for F
where
    F: Fn(&mut [f32], Context, &T),
{
    fn apply(&self, dst: &mut [f32], context: Context, state: &T) {
        (*self)(dst, context, state);
    }
}

pub struct DynamicProcessorNumberSource<T: SoundState, F: StateFunction<T>> {
    data: Arc<DynamicSoundProcessorData<T>>,
    function: F,
}

impl<T: SoundState, F: StateFunction<T>> DynamicProcessorNumberSource<T, F> {
    pub(super) fn new(
        data: Arc<DynamicSoundProcessorData<T>>,
        function: F,
    ) -> DynamicProcessorNumberSource<T, F> {
        DynamicProcessorNumberSource { data, function }
    }
}

impl<T: SoundState, F: StateFunction<T>> NumberSource for DynamicProcessorNumberSource<T, F> {
    fn eval(&self, dst: &mut [f32], context: Context) {
        let state = context.dynamic_sound_processor_state(&self.data);
        self.function.apply(dst, context, &state.read());
    }
}

pub struct StaticProcessorNumberSource<T: SoundState, F: StateFunction<T>> {
    data: Arc<StaticSoundProcessorData<T>>,
    function: F,
}

impl<T: SoundState, F: StateFunction<T>> StaticProcessorNumberSource<T, F> {
    pub(super) fn new(
        data: Arc<StaticSoundProcessorData<T>>,
        function: F,
    ) -> StaticProcessorNumberSource<T, F> {
        StaticProcessorNumberSource { data, function }
    }
}

impl<T: SoundState, F: StateFunction<T>> NumberSource for StaticProcessorNumberSource<T, F> {
    fn eval(&self, dst: &mut [f32], context: Context) {
        let state = context.static_sound_processor_state(&self.data);
        self.function.apply(dst, context, &state.read());
    }
}

pub struct SingleInputNumberSource<F: StateFunction<EmptyState>> {
    handle: SingleSoundInputHandle,
    function: F,
}

impl<F: StateFunction<EmptyState>> SingleInputNumberSource<F> {
    pub(super) fn new(handle: SingleSoundInputHandle, function: F) -> SingleInputNumberSource<F> {
        SingleInputNumberSource { handle, function }
    }
}

impl<F: StateFunction<EmptyState>> NumberSource for SingleInputNumberSource<F> {
    fn eval(&self, dst: &mut [f32], context: Context) {
        let state = context.single_input_state(&self.handle);
        self.function.apply(dst, context, &state.read());
    }
}

// TODO: elaborate to allow the current key to be passed by reference to the function in addition to the state
pub struct KeyedSoundInputNumberSource<K: Key, T: SoundState, F: StateFunction<T>> {
    handle: KeyedSoundInputHandle<K, T>,
    function: F,
}

impl<K: Key, T: SoundState, F: StateFunction<T>> KeyedSoundInputNumberSource<K, T, F> {
    pub(super) fn new(
        handle: KeyedSoundInputHandle<K, T>,
        function: F,
    ) -> KeyedSoundInputNumberSource<K, T, F> {
        KeyedSoundInputNumberSource { handle, function }
    }
}

impl<K: Key, T: SoundState, F: StateFunction<T>> NumberSource
    for KeyedSoundInputNumberSource<K, T, F>
{
    fn eval(&self, dst: &mut [f32], context: Context) {
        let state = context.keyed_input_state(&self.handle);
        self.function.apply(dst, context, &state.read());
    }
}
