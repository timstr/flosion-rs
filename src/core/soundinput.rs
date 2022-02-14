use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};

use parking_lot::{Mutex, RwLock};

use super::{
    gridspan::GridSpan,
    key::Key,
    keyrange::KeyRange,
    soundengine::{SoundEngineTools, StateOperation},
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
    keys: RwLock<KeyRange<K>>,
    options: InputOptions,
    message_receiver: Mutex<Receiver<KeyedSoundInputMessage<K>>>,
    id: SoundInputId,
}

impl<K: Key, T: SoundState> KeyedSoundInput<K, T> {
    pub(super) fn new(
        id: SoundInputId,
        options: InputOptions,
    ) -> (Arc<KeyedSoundInput<K, T>>, KeyedSoundInputHandle<K, T>) {
        let (tx, rx) = channel();
        let input = Arc::new(KeyedSoundInput {
            state_table: RwLock::new(KeyedStateTable::<T>::new()),
            keys: RwLock::new(KeyRange::new()),
            options,
            message_receiver: Mutex::new(rx),
            id,
        });
        let handle = KeyedSoundInputHandle {
            input: Arc::clone(&input),
            message_sender: Mutex::new(tx),
        };
        (input, handle)
    }

    pub(super) fn get_state<'a>(
        &'a self,
        state_index: usize,
        key_index: usize,
    ) -> StateTableLock<'a, T> {
        StateTableLock::new_keyed(self.state_table.read(), state_index, key_index)
    }

    pub(super) fn flush_messages(&self, own_id: SoundInputId, tools: &mut SoundEngineTools) {
        let rcv = self.message_receiver.lock();
        while let Ok(msg) = rcv.try_recv() {
            match msg {
                KeyedSoundInputMessage::AddKey { key } => {
                    let i = self.keys.write().insert_key(key);
                    let gs = self.state_table.write().insert_key(i);
                    tools.propagate_input_key_change(own_id, gs, StateOperation::Insert);
                }
                KeyedSoundInputMessage::RemoveKey { index } => {
                    self.keys.write().erase_key(index);
                    let gs = self.state_table.write().erase_key(index);
                    tools.propagate_input_key_change(own_id, gs, StateOperation::Erase);
                }
            }
        }
    }
}

pub trait SoundInputWrapper: Sync + Send {
    fn id(&self) -> SoundInputId;

    fn options(&self) -> InputOptions;

    fn num_keys(&self) -> usize;

    fn insert_states(&self, gs: GridSpan) -> GridSpan;

    fn erase_states(&self, gs: GridSpan) -> GridSpan;

    fn flush_message(&self, tools: &mut SoundEngineTools);
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

    fn insert_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().insert_states(gs);
        gs
    }

    fn erase_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().erase_states(gs);
        gs
    }

    fn flush_message(&self, _tools: &mut SoundEngineTools) {
        // Nothing to do
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

    fn insert_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().insert_states(gs)
    }
    fn erase_states(&self, gs: GridSpan) -> GridSpan {
        self.state_table.write().erase_states(gs)
    }

    fn flush_message(&self, tools: &mut SoundEngineTools) {
        self.flush_messages(self.id, tools)
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
    message_sender: Mutex<Sender<KeyedSoundInputMessage<K>>>,
}

impl<K: Key, T: SoundState> KeyedSoundInputHandle<K, T> {
    pub fn id(&self) -> SoundInputId {
        self.input.id()
    }

    pub(super) fn input(&self) -> &KeyedSoundInput<K, T> {
        &*self.input
    }

    pub fn add_key(&mut self, key: Arc<K>) {
        self.message_sender
            .lock()
            .send(KeyedSoundInputMessage::AddKey { key })
            .unwrap();
    }

    pub fn remove_key(&mut self, index: usize) {
        self.message_sender
            .lock()
            .send(KeyedSoundInputMessage::RemoveKey { index })
            .unwrap();
    }

    pub(super) fn clone(&self) -> KeyedSoundInputHandle<K, T> {
        KeyedSoundInputHandle {
            input: Arc::clone(&self.input),
            message_sender: Mutex::new(self.message_sender.lock().clone()),
        }
    }
}
