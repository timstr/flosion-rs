use std::{
    marker,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use super::{
    gridspan::GridSpan,
    key::Key,
    keyrange::KeyRange,
    soundengine::{SoundEngineTools, StateOperation},
    soundstate::{EmptyState, SoundState},
    statetable::{KeyedStateTable, StateTable},
    uniqueid::UniqueId,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundInputId(usize);

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
    state_table: StateTable<EmptyState>,
    options: InputOptions,
    id: SoundInputId,
}

impl SingleSoundInput {
    pub fn new(
        id: SoundInputId,
        options: InputOptions,
    ) -> (Box<SingleSoundInput>, SingleSoundInputHandle) {
        let input = Box::new(SingleSoundInput {
            state_table: StateTable::new(),
            options,
            id,
        });
        let handle = SingleSoundInputHandle::new(&input);
        (input, handle)
    }
}

pub enum KeyedSoundInputMessage<K: Key> {
    AddKey(Arc<K>),
    RemoveKey(usize),
}

pub struct KeyedSoundInput<K: Key, T: SoundState> {
    state_table: KeyedStateTable<T>,
    keys: KeyRange<K>,
    options: InputOptions,
    message_receiver: Receiver<KeyedSoundInputMessage<K>>,
    id: SoundInputId,
}

impl<K: Key, T: SoundState> KeyedSoundInput<K, T> {
    pub fn new(
        id: SoundInputId,
        options: InputOptions,
    ) -> (Box<KeyedSoundInput<K, T>>, KeyedSoundInputHandle<K, T>) {
        let (tx, rx) = channel();
        let input = Box::new(KeyedSoundInput {
            state_table: KeyedStateTable::<T>::new(),
            keys: KeyRange::new(),
            options,
            message_receiver: rx,
            id,
        });
        let handle = KeyedSoundInputHandle {
            id,
            local_keys: input.keys.clone(),
            message_sender: tx,
            _marker: marker::PhantomData,
        };
        (input, handle)
    }

    pub fn flush_messages(&mut self, own_id: SoundInputId, tools: &mut SoundEngineTools) {
        while let Ok(msg) = self.message_receiver.try_recv() {
            match msg {
                KeyedSoundInputMessage::AddKey(k) => {
                    let i = self.keys.insert_key(k);
                    let gs = self.state_table.insert_key(i);
                    tools.propagate_input_key_change(own_id, gs, StateOperation::Insert);
                }
                KeyedSoundInputMessage::RemoveKey(i) => {
                    self.keys.erase_key(i);
                    let gs = self.state_table.erase_key(i);
                    tools.propagate_input_key_change(own_id, gs, StateOperation::Erase);
                }
            }
        }
    }
}

pub trait SoundInputWrapper: Send {
    fn id(&self) -> SoundInputId;

    fn options(&self) -> InputOptions;

    fn num_keys(&self) -> usize;

    fn insert_states(&mut self, gs: GridSpan) -> GridSpan;

    fn erase_states(&mut self, gs: GridSpan) -> GridSpan;

    fn flush_message(&mut self, tools: &mut SoundEngineTools);
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

    fn insert_states(&mut self, gs: GridSpan) -> GridSpan {
        self.state_table.insert_states(gs);
        gs
    }

    fn erase_states(&mut self, gs: GridSpan) -> GridSpan {
        self.state_table.erase_states(gs);
        gs
    }

    fn flush_message(&mut self, _tools: &mut SoundEngineTools) {
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
        self.state_table.num_keys()
    }

    fn insert_states(&mut self, gs: GridSpan) -> GridSpan {
        self.state_table.insert_states(gs)
    }
    fn erase_states(&mut self, gs: GridSpan) -> GridSpan {
        self.state_table.erase_states(gs)
    }

    fn flush_message(&mut self, tools: &mut SoundEngineTools) {
        self.flush_messages(self.id, tools)
    }
}

pub struct SingleSoundInputHandle {
    id: SoundInputId,
}

impl SingleSoundInputHandle {
    fn new(input: &SingleSoundInput) -> SingleSoundInputHandle {
        SingleSoundInputHandle { id: input.id }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }
}

pub struct KeyedSoundInputHandle<K: Key, T: SoundState> {
    id: SoundInputId,
    local_keys: KeyRange<K>,
    message_sender: Sender<KeyedSoundInputMessage<K>>,
    _marker: marker::PhantomData<T>,
}

impl<K: Key, T: SoundState> KeyedSoundInputHandle<K, T> {
    pub fn id(&self) -> SoundInputId {
        self.id
    }

    pub fn add_key(&mut self, key: Arc<K>) {
        self.local_keys.insert_key(Arc::clone(&key));
        self.message_sender
            .send(KeyedSoundInputMessage::AddKey(key))
            .unwrap();
    }

    pub fn remove_key(&mut self, index: usize) {
        self.local_keys.erase_key(index);
        self.message_sender
            .send(KeyedSoundInputMessage::RemoveKey(index))
            .unwrap();
    }

    pub fn keys(&self) -> &[Arc<K>] {
        self.local_keys.keys()
    }
}
