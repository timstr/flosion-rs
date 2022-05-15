use std::sync::Arc;

use parking_lot::{MappedRwLockReadGuard, RwLock};

use super::{
    gridspan::GridSpan,
    key::{Key, TypeErasedKey},
    keyrange::KeyRange,
    resultfuture::ResultFuture,
    soundprocessortools::SoundProcessorTools,
    soundstate::{EmptyState, SoundState},
    statetable::{KeyedTable, Table, TableLock},
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

struct InputTiming {
    elapsed_chunks: usize,
    sample_offset: usize,
}

impl InputTiming {
    fn reset(&mut self) {
        self.elapsed_chunks = 0;
        self.sample_offset = 0;
    }
}

impl Default for InputTiming {
    fn default() -> InputTiming {
        InputTiming {
            elapsed_chunks: 0,
            sample_offset: 0,
        }
    }
}

pub struct SoundInputState<T: SoundState> {
    state: T,
    timing: InputTiming,
}

impl<T: SoundState> SoundInputState<T> {
    pub fn state(&self) -> &T {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    pub fn reset(&mut self) {
        self.state.reset();
        self.timing.reset();
    }
}

impl<T: SoundState> Default for SoundInputState<T> {
    fn default() -> SoundInputState<T> {
        SoundInputState {
            state: T::default(),
            timing: InputTiming::default(),
        }
    }
}

pub struct SingleSoundInput {
    state_table: RwLock<Table<SoundInputState<EmptyState>>>,
    options: InputOptions,
    id: SoundInputId,
}

impl SingleSoundInput {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
    ) -> (Arc<SingleSoundInput>, SingleSoundInputHandle) {
        let input = Arc::new(SingleSoundInput {
            state_table: RwLock::new(Table::new()),
            options,
            id,
        });
        let handle = SingleSoundInputHandle::new(Arc::clone(&input));
        (input, handle)
    }

    pub fn options(&self) -> &InputOptions {
        &self.options
    }

    pub(super) fn get_state<'a>(
        &'a self,
        index: usize,
    ) -> TableLock<'a, SoundInputState<EmptyState>> {
        TableLock::new(self.state_table.read(), index)
    }
}

// TODO: consider adding OutBoundResult to each message and returning futures to the client
pub enum KeyedSoundInputMessage<K: Key> {
    AddKey { key: Arc<K> },
    RemoveKey { index: usize },
}

pub struct KeyedSoundInput<K: Key, T: SoundState> {
    state_table: RwLock<KeyedTable<SoundInputState<T>>>,
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
            state_table: RwLock::new(KeyedTable::new()),
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
    ) -> TableLock<'a, SoundInputState<T>> {
        TableLock::new_keyed(self.state_table.read(), state_index, key_index)
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

    fn reset_state(&self, state_index: usize, key_index: usize);

    fn get_state_index(&self, state_index: usize, key_index: usize) -> usize {
        debug_assert!(state_index < self.num_parent_states());
        debug_assert!(key_index < self.num_keys());
        state_index * self.num_keys() + key_index
    }
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
        self.state_table.write().insert(gs);
        gs
    }

    fn erase_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().erase(gs);
        gs
    }

    fn reset_state(&self, state_index: usize, key_index: usize) {
        debug_assert!(key_index == 0);
        debug_assert!(self.num_keys() == 1);
        self.state_table.read().get(state_index).write().reset();
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
        self.state_table.read().num_parent_items()
    }

    fn insert_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().insert_items(gs)
    }

    fn erase_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().erase_items(gs)
    }

    fn reset_state(&self, state_index: usize, key_index: usize) {
        self.state_table
            .read()
            .get(state_index, key_index)
            .write()
            .reset();
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
