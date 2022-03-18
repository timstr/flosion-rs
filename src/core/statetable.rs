use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::gridspan::GridSpan;
use crate::core::soundinput::SoundInputId;
use crate::core::soundstate::SoundState;

pub struct StateTable<T: SoundState> {
    data: Vec<RwLock<T>>,
}

impl<T: SoundState> StateTable<T> {
    pub fn new() -> StateTable<T> {
        StateTable { data: Vec::new() }
    }

    pub fn total_size(&self) -> usize {
        self.data.len()
    }

    pub fn insert_states(&mut self, span: GridSpan) {
        self.data = span.insert_with(std::mem::take(&mut self.data), || RwLock::new(T::default()));
    }

    pub fn erase_states(&mut self, span: GridSpan) {
        self.data = span.erase(std::mem::take(&mut self.data));
    }

    pub fn reset_states(&self, span: GridSpan) {
        for r in 0..span.num_rows() {
            let row_begin = span.start_index() + (r * span.row_stride());
            let row_end = row_begin + span.items_per_row();
            for s in &self.data[row_begin..row_end] {
                s.write().reset();
            }
        }
    }

    pub fn get_state(&self, index: usize) -> &RwLock<T> {
        &self.data[index]
    }
}

pub struct KeyedStateTable<T: SoundState> {
    data: Vec<RwLock<T>>,
    num_keys: usize,
    num_parent_states: usize,
}

impl<T: SoundState> KeyedStateTable<T> {
    pub fn new() -> KeyedStateTable<T> {
        KeyedStateTable {
            data: Vec::new(),
            num_keys: 0,
            num_parent_states: 0,
        }
    }

    pub fn num_keys(&self) -> usize {
        self.num_keys
    }

    pub fn num_parent_states(&self) -> usize {
        self.num_parent_states
    }

    pub fn insert_key(&mut self, index: usize) -> GridSpan {
        let gs = GridSpan::new(index, 1, self.num_keys, self.num_parent_states);
        self.data = gs.insert_with(std::mem::take(&mut self.data), || RwLock::new(T::default()));
        self.num_keys += 1;
        gs
    }

    pub fn erase_key(&mut self, index: usize) -> GridSpan {
        debug_assert!(index < self.num_keys);
        let gs = GridSpan::new(index, 1, self.num_keys, self.num_parent_states);
        self.data = gs.erase(std::mem::take(&mut self.data));
        self.num_keys -= 1;
        gs
    }

    pub fn reset_key(&self, state_index: usize, key_index: usize) -> GridSpan {
        debug_assert!(
            state_index < self.num_parent_states,
            "State index {} is out of bounds for {} parent states",
            state_index,
            self.num_parent_states
        );
        debug_assert!(key_index < self.num_keys);
        let i = (state_index * self.num_keys) + key_index;
        self.data[i].write().reset();
        GridSpan::new_contiguous(i, 1)
    }

    pub fn insert_states(&mut self, span: GridSpan) -> GridSpan {
        self.num_parent_states += span.num_items();
        let span = span.inflate(self.num_keys);
        self.data = span.insert_with(std::mem::take(&mut self.data), || RwLock::new(T::default()));
        span
    }

    pub fn erase_states(&mut self, span: GridSpan) -> GridSpan {
        debug_assert!(span.num_items() <= self.num_parent_states);
        self.num_parent_states -= span.num_items();
        let span = span.inflate(self.num_keys);
        self.data = span.erase(std::mem::take(&mut self.data));
        span
    }

    pub fn reset_states(&self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.num_keys);
        // TODO: this is silly. Let the GridSpan create an iterator of indices instead
        for (i, s) in self.data.iter().enumerate() {
            if span.contains(i) {
                s.write().reset();
            }
        }
        span
    }

    pub fn get_state(&self, state_index: usize, key_index: usize) -> &RwLock<T> {
        debug_assert!(key_index < self.num_keys);
        &self.data[self.num_keys * state_index + key_index]
    }
}

struct StateTableSlice {
    index: usize,
    count: usize,
}

pub struct StateTablePartition {
    offsets: Vec<(SoundInputId, StateTableSlice)>,
    is_static: bool,
}

impl StateTablePartition {
    pub fn new(is_static: bool) -> StateTablePartition {
        StateTablePartition {
            offsets: Vec::new(),
            is_static,
        }
    }

    pub fn get_index(&self, input_id: SoundInputId, input_state_index: usize) -> usize {
        debug_assert!(self.offsets.iter().find(|(i, _)| *i == input_id).is_some());
        if self.is_static {
            debug_assert!({
                let (_, s) = self.offsets.iter().find(|(i, _)| *i == input_id).unwrap();
                s.index == 0 && s.count == 1
            });
            return 0;
        }
        for (i, s) in &self.offsets {
            if *i == input_id {
                debug_assert!(input_state_index < s.count);
                return s.index + input_state_index;
            }
        }
        panic!();
    }

    pub fn get_span(&self, input_id: SoundInputId, input_span: GridSpan) -> GridSpan {
        debug_assert!(self.offsets.iter().find(|(i, _)| *i == input_id).is_some());
        if self.is_static {
            debug_assert!({
                let (_, s) = self.offsets.iter().find(|(i, _)| *i == input_id).unwrap();
                s.index == 0 && s.count == 1
            });
            debug_assert!(input_span.start_index() == 0);
            debug_assert!(input_span.items_per_row() == 1);
            debug_assert!(input_span.num_rows() == 1);
            return GridSpan::new_empty();
        }
        for (i, s) in &self.offsets {
            if *i == input_id {
                debug_assert!(input_span.start_index() < s.count);
                debug_assert!(input_span.last_index() < s.count);
                return input_span.offset(s.index);
            }
        }
        panic!();
    }

    pub fn total_size(&self) -> usize {
        if self.is_static {
            return 1;
        }
        let mut acc: usize = 0;
        for (_, s) in &self.offsets {
            assert_eq!(s.index, acc);
            acc += s.count;
        }
        acc
    }

    // Returns the span of states to insert
    pub fn add_dst(&mut self, input_id: SoundInputId) {
        debug_assert!(self
            .offsets
            .iter()
            .find(|(id, _)| *id == input_id)
            .is_none());
        let index = if self.is_static { 0 } else { self.total_size() };
        let s = StateTableSlice { index, count: 0 };
        self.offsets.push((input_id, s));
    }

    // Returns the span of states to erase
    pub fn remove_dst(&mut self, input_id: SoundInputId) {
        assert_eq!(
            self.offsets.iter().filter(|(i, _)| *i == input_id).count(),
            1
        );
        let index = self
            .offsets
            .iter()
            .position(|(i, _)| *i == input_id)
            .unwrap();
        let o = self.offsets.remove(index);
        assert_eq!(o.1.count, 0);
    }

    // Returns the span of states to insert
    pub fn add_dst_states(&mut self, input_id: SoundInputId, span: GridSpan) -> GridSpan {
        if self.is_static {
            let (_, s) = self
                .offsets
                .iter_mut()
                .find(|(i, _)| *i == input_id)
                .unwrap();
            debug_assert!(s.index == 0);
            debug_assert!(s.count == 0);
            debug_assert!(span.start_index() == 0);
            debug_assert!(span.items_per_row() == 1);
            debug_assert!(span.num_rows() == 1);
            s.count = 1;
            return GridSpan::new_empty();
        }
        let index = self
            .offsets
            .iter()
            .position(|(i, _)| *i == input_id)
            .unwrap();
        let new_items = span.num_items();
        let new_span;
        {
            let o = &mut self.offsets[index];
            o.1.count += new_items;
            new_span = span.offset(o.1.index);
        }
        let next_index = index + 1;
        if next_index < self.offsets.len() {
            for (_, s) in self.offsets[next_index..].iter_mut() {
                s.index += new_items;
            }
        }
        new_span
    }
    // Returns the span of states to erase
    pub fn remove_dst_states(&mut self, input_id: SoundInputId, span: GridSpan) -> GridSpan {
        if self.is_static {
            let (_, s) = self
                .offsets
                .iter_mut()
                .find(|(i, _)| *i == input_id)
                .unwrap();
            debug_assert!(s.index == 0);
            debug_assert!(s.count == 1);
            debug_assert!(span.start_index() == 0);
            debug_assert!(span.items_per_row() == 1);
            debug_assert!(span.num_rows() == 1);
            s.count = 0;
            return GridSpan::new_empty();
        }
        let index = self
            .offsets
            .iter()
            .position(|(i, _)| *i == input_id)
            .unwrap();
        let new_items = span.num_items();
        let new_span;
        {
            let o = &mut self.offsets[index];
            o.1.count -= new_items;
            new_span = span.offset(o.1.index);
        }
        let next_index = index + 1;
        if next_index < self.offsets.len() {
            for (_, s) in self.offsets[next_index..].iter_mut() {
                s.index -= new_items;
            }
        }
        new_span
    }
}

pub struct StateTableLock<'a, T: SoundState> {
    lock: MappedRwLockReadGuard<'a, RwLock<T>>,
}

impl<'a, T: SoundState> StateTableLock<'a, T> {
    pub fn new(
        state_table: RwLockReadGuard<'a, StateTable<T>>,
        index: usize,
    ) -> StateTableLock<'a, T> {
        StateTableLock {
            lock: RwLockReadGuard::map(state_table, |st| st.get_state(index)),
        }
    }

    pub fn new_keyed(
        state_table: RwLockReadGuard<'a, KeyedStateTable<T>>,
        state_index: usize,
        key_index: usize,
    ) -> StateTableLock<'a, T> {
        StateTableLock {
            lock: RwLockReadGuard::map(state_table, |st| st.get_state(state_index, key_index)),
        }
    }

    pub fn read(&'a self) -> RwLockReadGuard<'a, T> {
        self.lock.read()
    }

    pub fn write(&'a self) -> RwLockWriteGuard<'a, T> {
        self.lock.write()
    }
}
