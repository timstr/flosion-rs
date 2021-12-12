use crate::sound::gridspan::GridSpan;
use crate::sound::soundstate::{EmptyState, SoundState};
use crate::sound::statetable::{KeyedStateTable, StateTable};
use crate::sound::uniqueid::IdGenerator;
use crate::sound::uniqueid::UniqueId;
use std::cell::RefCell;
use std::rc::Rc;

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
    id: SoundInputId,
}

impl SingleSoundInput {
    pub fn new(id_gen: &mut IdGenerator<SoundInputId>) -> SingleSoundInput {
        SingleSoundInput {
            state_table: StateTable::new(),
            id: id_gen.next_id(),
        }
    }
}

pub struct KeyedSoundInput<K: Ord, T: SoundState> {
    state_table: KeyedStateTable<K, T>,
    id: SoundInputId,
}

impl<K: Ord, T: SoundState> KeyedSoundInput<K, T> {
    pub fn new(id_gen: &mut IdGenerator<SoundInputId>) -> KeyedSoundInput<K, T> {
        KeyedSoundInput {
            state_table: KeyedStateTable::<K, T>::new(),
            id: id_gen.next_id(),
        }
    }
}

pub trait SoundInputWrapper {
    fn id(&self) -> SoundInputId;

    fn num_keys(&self) -> usize;

    fn insert_states(&mut self, gs: GridSpan) -> GridSpan;

    fn erase_states(&mut self, gs: GridSpan) -> GridSpan;
}

impl SoundInputWrapper for SingleSoundInput {
    fn id(&self) -> SoundInputId {
        self.id
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
}

impl<K: Ord, T: SoundState> SoundInputWrapper for KeyedSoundInput<K, T> {
    fn id(&self) -> SoundInputId {
        self.id
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
}

pub struct SingleSoundInputHandle {
    id: SoundInputId,
    input: Rc<RefCell<SingleSoundInput>>,
}

impl SingleSoundInputHandle {
    pub(in crate::sound) fn new(input: Rc<RefCell<SingleSoundInput>>) -> SingleSoundInputHandle {
        let id = input.borrow().id();
        SingleSoundInputHandle { id, input }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }
}

pub struct KeyedSoundInputHandle<K: Ord, T: SoundState> {
    id: SoundInputId,
    input: Rc<RefCell<KeyedSoundInput<K, T>>>,
}

impl<K: Ord, T: SoundState> KeyedSoundInputHandle<K, T> {
    pub(in crate::sound) fn new(
        input: Rc<RefCell<KeyedSoundInput<K, T>>>,
    ) -> KeyedSoundInputHandle<K, T> {
        let id = input.borrow().id();
        KeyedSoundInputHandle { id, input }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }
}
