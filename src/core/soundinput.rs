use std::sync::Arc;

use parking_lot::{MappedRwLockReadGuard, RwLock};

use crate::core::soundchunk::CHUNK_SIZE;

use super::{
    gridspan::GridSpan,
    key::{Key, TypeErasedKey},
    keyrange::KeyRange,
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

#[derive(Clone, Copy)]
pub struct InputTiming {
    elapsed_chunks: usize,
    sample_offset: usize,
    needs_reset: bool,
}

impl InputTiming {
    fn require_reset(&mut self) {
        self.needs_reset = true;
    }

    fn needs_reset(&self) -> bool {
        self.needs_reset
    }

    fn advance_one_chunk(&mut self) -> () {
        debug_assert!(!self.needs_reset);
        self.elapsed_chunks += 1;
    }

    fn reset(&mut self, sample_offset: usize) {
        debug_assert!(sample_offset < CHUNK_SIZE);
        self.elapsed_chunks = 0;
        self.sample_offset = sample_offset;
        self.needs_reset = false;
    }

    pub fn elapsed_chunks(&self) -> usize {
        self.elapsed_chunks
    }

    pub fn sample_offset(&self) -> usize {
        self.sample_offset
    }
}

impl Default for InputTiming {
    fn default() -> InputTiming {
        InputTiming {
            elapsed_chunks: 0,
            sample_offset: 0,
            needs_reset: true,
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

    fn require_reset(&mut self) {
        self.timing.require_reset();
    }

    fn needs_reset(&self) -> bool {
        self.timing.needs_reset()
    }

    pub fn reset(&mut self, sample_offset: usize) {
        self.state.reset();
        self.timing.reset(sample_offset);
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

    fn require_reset_states(&self, state_index: usize);

    fn reset_state(&self, state_index: usize, key_index: usize, sample_offset: usize);

    fn state_needs_reset(&self, state_index: usize, key_index: usize) -> bool;

    fn get_state_time(&self, state_index: usize, key_index: usize) -> InputTiming;

    fn get_state_index(&self, state_index: usize, key_index: usize) -> usize {
        debug_assert!(state_index < self.num_parent_states());
        debug_assert!(key_index < self.num_keys());
        state_index * self.num_keys() + key_index
    }

    fn advance_timing_one_chunk(&self, state_index: usize, key_index: usize);
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

    fn require_reset_states(&self, state_index: usize) {
        self.state_table
            .read()
            .get(state_index)
            .write()
            .require_reset();
    }

    fn state_needs_reset(&self, state_index: usize, key_index: usize) -> bool {
        debug_assert!(key_index == 0);
        debug_assert!(self.num_keys() == 1);
        self.state_table
            .read()
            .get(state_index)
            .read()
            .needs_reset()
    }

    fn reset_state(&self, state_index: usize, key_index: usize, sample_offset: usize) {
        debug_assert!(key_index == 0);
        debug_assert!(self.num_keys() == 1);
        self.state_table
            .read()
            .get(state_index)
            .write()
            .reset(sample_offset);
    }

    fn get_state_time(&self, state_index: usize, key_index: usize) -> InputTiming {
        debug_assert!(key_index == 0);
        debug_assert!(self.num_keys() == 1);
        self.state_table.read().get(state_index).read().timing
    }

    fn advance_timing_one_chunk(&self, state_index: usize, key_index: usize) {
        debug_assert!(key_index == 0);
        debug_assert!(self.num_keys() == 1);
        self.state_table
            .read()
            .get(state_index)
            .write()
            .timing
            .advance_one_chunk();
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

    fn require_reset_states(&self, state_index: usize) {
        let st = self.state_table.read();
        for k in 0..self.num_keys() {
            st.get(state_index, k).write().require_reset();
        }
    }

    fn state_needs_reset(&self, state_index: usize, key_index: usize) -> bool {
        self.state_table
            .read()
            .get(state_index, key_index)
            .read()
            .needs_reset()
    }

    fn reset_state(&self, state_index: usize, key_index: usize, sample_offset: usize) {
        self.state_table
            .read()
            .get(state_index, key_index)
            .write()
            .reset(sample_offset);
    }

    fn get_state_time(&self, state_index: usize, key_index: usize) -> InputTiming {
        self.state_table
            .read()
            .get(state_index, key_index)
            .read()
            .timing
    }

    fn advance_timing_one_chunk(&self, state_index: usize, key_index: usize) {
        self.state_table
            .read()
            .get(state_index, key_index)
            .write()
            .timing
            .advance_one_chunk();
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

    pub fn add_key<TT: SoundState>(&mut self, key: K, tools: &mut SoundProcessorTools<TT>) {
        tools.add_keyed_input_key(self.input.id(), TypeErasedKey::new(key))
    }

    pub fn remove_key<TT: SoundState>(
        &mut self,
        index: usize,
        tools: &mut SoundProcessorTools<TT>,
    ) {
        tools.remove_keyed_input_key(self.input.id(), index)
    }

    pub(super) fn clone(&self) -> KeyedSoundInputHandle<K, T> {
        KeyedSoundInputHandle {
            input: Arc::clone(&self.input),
        }
    }
}
