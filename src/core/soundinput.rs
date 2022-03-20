use std::sync::Arc;

use parking_lot::{MappedRwLockReadGuard, RwLock};

use super::{
    gridspan::GridSpan,
    key::{Key, TypeErasedKey},
    keyrange::KeyRange,
    resultfuture::ResultFuture,
    soundprocessortools::SoundProcessorTools,
    soundstate::{EmptyState, SoundState},
    statetable::{KeyedStateTable, StateTable, StateTableLock},
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundInputId(pub usize);

impl Default for SoundInputId {
    fn default() -> SoundInputId {
        SoundInputId(1)
    }
}

impl UniqueId for SoundInputId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundInputId {
        SoundInputId(self.0 + 1)
    }
}

#[derive(Copy, Clone)]
pub struct InputOptions {
    // Will the input ever be paused or reset by the sound processor?
    pub interruptible: bool,

    // Will the input's speed of time always be the same as the sound processor's?
    pub realtime: bool,
}

pub struct SingleSoundInput {
    state_table: RwLock<StateTable<EmptyState>>,
    options: InputOptions,
    id: SoundInputId,
}

impl SingleSoundInput {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
    ) -> (Arc<SingleSoundInput>, SingleSoundInputHandle) {
        let input = Arc::new(SingleSoundInput {
            state_table: RwLock::new(StateTable::new()),
            options,
            id,
        });
        let handle = SingleSoundInputHandle::new(Arc::clone(&input));
        (input, handle)
    }

    pub fn options(&self) -> &InputOptions {
        &self.options
    }

    pub(super) fn get_state<'a>(&'a self, index: usize) -> StateTableLock<'a, EmptyState> {
        StateTableLock::new(self.state_table.read(), index)
    }
}

// TODO: consider adding OutBoundResult to each message and returning futures to the client
pub enum KeyedSoundInputMessage<K: Key> {
    AddKey { key: Arc<K> },
    RemoveKey { index: usize },
}

pub struct KeyedSoundInput<K: Key, T: SoundState> {
    state_table: RwLock<KeyedStateTable<T>>,
    keys: KeyRange<K>,
    options: InputOptions,
    id: SoundInputId,
}

impl<K: Key, T: SoundState> KeyedSoundInput<K, T> {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
    ) -> (Arc<KeyedSoundInput<K, T>>, KeyedSoundInputHandle<K, T>) {
        let input = Arc::new(KeyedSoundInput {
            state_table: RwLock::new(KeyedStateTable::<T>::new()),
            keys: KeyRange::new(),
            options,
            id,
        });
        let handle = KeyedSoundInputHandle {
            input: Arc::clone(&input),
        };
        (input, handle)
    }

    pub fn options(&self) -> &InputOptions {
        &self.options
    }

    pub(super) fn get_state<'a>(
        &'a self,
        state_index: usize,
        key_index: usize,
    ) -> StateTableLock<'a, T> {
        StateTableLock::new_keyed(self.state_table.read(), state_index, key_index)
    }

    pub(super) fn read_all_keys<'a>(&'a self) -> MappedRwLockReadGuard<'a, [K]> {
        self.keys.read_all_keys()
    }

    pub(super) fn read_key<'a>(&'a self, key_index: usize) -> MappedRwLockReadGuard<'a, K> {
        self.keys.read_key(key_index)
    }
}

pub trait SoundInputWrapper: Sync + Send {
    fn id(&self) -> SoundInputId;

    fn options(&self) -> InputOptions;

    fn num_keys(&self) -> usize;

    fn insert_key(&self, key: TypeErasedKey) -> GridSpan;

    fn erase_key(&self, key_index: usize) -> GridSpan;

    fn num_parent_states(&self) -> usize;

    fn insert_states(&self, gs: GridSpan) -> GridSpan;

    fn erase_states(&self, gs: GridSpan) -> GridSpan;

    fn reset_states(&self, gs: GridSpan) -> GridSpan;

    fn reset_key(&self, state_index: usize, key_index: usize) -> GridSpan;
}

impl SoundInputWrapper for SingleSoundInput {
    fn id(&self) -> SoundInputId {
        self.id
    }

    fn options(&self) -> InputOptions {
        self.options
    }

    fn num_keys(&self) -> usize {
        1
    }

    fn insert_key(&self, _key: TypeErasedKey) -> GridSpan {
        panic!();
    }

    fn erase_key(&self, _key_index: usize) -> GridSpan {
        panic!()
    }

    fn num_parent_states(&self) -> usize {
        self.state_table.read().total_size()
    }

    fn insert_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().insert_states(gs);
        gs
    }

    fn erase_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().erase_states(gs);
        gs
    }

    fn reset_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.read().reset_states(gs);
        gs
    }

    fn reset_key(&self, state_index: usize, key_index: usize) -> GridSpan {
        debug_assert!(key_index == 0);
        let gs = GridSpan::new_contiguous(state_index, 1);
        self.state_table.read().reset_states(gs);
        gs
    }
}

impl<K: Key, T: SoundState> SoundInputWrapper for KeyedSoundInput<K, T> {
    fn id(&self) -> SoundInputId {
        self.id
    }

    fn options(&self) -> InputOptions {
        self.options
    }

    fn num_keys(&self) -> usize {
        self.state_table.read().num_keys()
    }

    fn insert_key(&self, key: TypeErasedKey) -> GridSpan {
        let k = key.into::<K>();
        let i = self.keys.insert_key(k);
        let gs = self.state_table.write().insert_key(i);
        gs
    }

    fn erase_key(&self, key_index: usize) -> GridSpan {
        self.keys.erase_key(key_index);
        let gs = self.state_table.write().erase_key(key_index);
        gs
    }

    fn num_parent_states(&self) -> usize {
        self.state_table.read().num_parent_states()
    }

    fn insert_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().insert_states(gs)
    }

    fn erase_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().erase_states(gs)
    }

    fn reset_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.read().reset_states(gs)
    }

    fn reset_key(&self, state_index: usize, key_index: usize) -> GridSpan {
        self.state_table.read().reset_key(state_index, key_index)
    }
}

pub struct SingleSoundInputHandle {
    input: Arc<SingleSoundInput>,
}

impl SingleSoundInputHandle {
    fn new(input: Arc<SingleSoundInput>) -> SingleSoundInputHandle {
        SingleSoundInputHandle { input }
    }

    pub fn id(&self) -> SoundInputId {
        self.input.id()
    }

    pub(super) fn input(&self) -> &SingleSoundInput {
        &*self.input
    }

    pub(super) fn clone(&self) -> SingleSoundInputHandle {
        SingleSoundInputHandle {
            input: Arc::clone(&self.input),
        }
    }
}

pub struct KeyedSoundInputHandle<K: Key, T: SoundState> {
    input: Arc<KeyedSoundInput<K, T>>,
}

impl<K: Key, T: SoundState> KeyedSoundInputHandle<K, T> {
    pub fn id(&self) -> SoundInputId {
        self.input.id()
    }

    pub(super) fn input(&self) -> &KeyedSoundInput<K, T> {
        &*self.input
    }

    pub fn num_keys(&self) -> usize {
        self.input.num_keys()
    }

    pub fn add_key<TT: SoundState>(
        &mut self,
        key: K,
        tools: &mut SoundProcessorTools<TT>,
    ) -> ResultFuture<(), ()> {
        tools.add_keyed_input_key(self.input.id(), TypeErasedKey::new(key))
    }

    pub fn remove_key<TT: SoundState>(
        &mut self,
        index: usize,
        tools: &mut SoundProcessorTools<TT>,
    ) -> ResultFuture<(), ()> {
        tools.remove_keyed_input_key(self.input.id(), index)
    }

    pub(super) fn clone(&self) -> KeyedSoundInputHandle<K, T> {
        KeyedSoundInputHandle {
            input: Arc::clone(&self.input),
        }
    }
}
