use crate::sound::gridspan::GridSpan;
use crate::sound::soundinput::SoundInputId;
use crate::sound::soundstate::SoundState;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

pub struct StateTable<T: SoundState> {
    data: Vec<RefCell<T>>,
}

impl<T: SoundState> StateTable<T> {
    pub fn new() -> StateTable<T> {
        StateTable { data: Vec::new() }
    }

    pub fn total_size(&self) -> usize {
        self.data.len()
    }

    pub fn insert_states(&mut self, span: GridSpan) {
        self.data = span.insert_with(
            std::mem::take(&mut self.data),
            || RefCell::new(T::default()),
        );
    }

    pub fn erase_states(&mut self, span: GridSpan) {
        self.data = span.erase(std::mem::take(&mut self.data));
    }

    pub fn reset_states(&self, span: GridSpan) {
        for r in 0..span.num_rows() {
            let row_begin = span.start_index() + (r * span.row_stride());
            let row_end = row_begin + span.items_per_row();
            for s in &self.data[row_begin..row_end] {
                s.borrow_mut().reset();
            }
        }
    }

    pub fn get_state<'a>(&'a self, index: usize) -> impl Deref<Target = T> + 'a {
        self.data[index].borrow()
    }

    pub fn get_state_mut<'a>(&'a self, index: usize) -> impl DerefMut<Target = T> + 'a {
        self.data[index].borrow_mut()
    }
}

pub struct KeyedStateTable<K: Ord, T: SoundState> {
    keys: Vec<K>,
    data: Vec<RefCell<T>>,
    num_parent_states: usize,
}

impl<K: Ord, T: SoundState> KeyedStateTable<K, T> {
    pub fn new() -> KeyedStateTable<K, T> {
        KeyedStateTable {
            keys: Vec::new(),
            data: Vec::new(),
            num_parent_states: 0,
        }
    }

    pub fn add_key(&mut self, key: K) -> GridSpan {
        let old_num_keys = self.keys.len();
        let index = self
            .keys
            .iter()
            .position(|k| *k > key)
            .unwrap_or(self.keys.len());
        self.keys.insert(index, key);
        let gs = GridSpan::new(
            index,
            old_num_keys,
            old_num_keys + 1,
            self.num_parent_states,
        );
        self.data = gs.insert_with(
            std::mem::take(&mut self.data),
            || RefCell::new(T::default()),
        );
        gs
    }

    pub fn remove_key(&mut self, key: K) -> GridSpan {
        let old_num_keys = self.keys.len();
        let index = self.keys.iter().position(|k| *k == key).unwrap();
        self.keys.remove(index);
        let gs = GridSpan::new(index, 1, old_num_keys, self.num_parent_states);
        self.data = gs.erase(std::mem::take(&mut self.data));
        gs
    }

    pub fn insert_states(&mut self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.keys.len());
        self.data = span.insert_with(
            std::mem::take(&mut self.data),
            || RefCell::new(T::default()),
        );
        span
    }

    pub fn erase_states(&mut self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.keys.len());
        self.data = span.erase(std::mem::take(&mut self.data));
        span
    }

    pub fn reset_states(&self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.keys.len());
        for (i, s) in self.data.iter().enumerate() {
            if span.contains(i) {
                s.borrow_mut().reset();
            }
        }
        span
    }

    pub fn get_state<'a>(
        &'a self,
        state_index: usize,
        key_index: usize,
    ) -> impl Deref<Target = T> + 'a {
        assert!(key_index < self.keys.len());
        self.data[self.keys.len() * state_index + key_index].borrow()
    }

    pub fn get_state_mut<'a>(
        &'a self,
        state_index: usize,
        key_index: usize,
    ) -> impl DerefMut<Target = T> + 'a {
        assert!(key_index < self.keys.len());
        self.data[self.keys.len() * state_index + key_index].borrow_mut()
    }
}

struct StateTableSlice {
    index: usize,
    count: usize,
}

pub struct StateTablePartition {
    offsets: Vec<(SoundInputId, StateTableSlice)>,
}

impl StateTablePartition {
    pub fn new() -> StateTablePartition {
        StateTablePartition {
            offsets: Vec::new(),
        }
    }

    pub fn get_index(&self, input_id: SoundInputId, input_state_index: usize) -> usize {
        assert!(self.offsets.iter().find(|(i, _)| *i == input_id).is_some());
        for (i, s) in self.offsets.iter() {
            if *i == input_id {
                assert!(input_state_index < s.count);
                return s.index + input_state_index;
            }
        }
        panic!();
    }

    pub fn get_span(&self, input_id: SoundInputId, input_span: GridSpan) -> GridSpan {
        assert!(self.offsets.iter().find(|(i, _)| *i == input_id).is_some());
        for (i, s) in self.offsets.iter() {
            if *i == input_id {
                assert!(input_span.start_index() < s.count);
                assert!(input_span.last_index() < s.count);
                return input_span.offset(s.index);
            }
        }
        panic!();
    }

    pub fn total_size(&self) -> usize {
        let mut acc: usize = 0;
        for (_, s) in self.offsets.iter() {
            assert_eq!(s.index, acc);
            acc += s.count;
        }
        acc
    }

    // Returns the span of states to insert
    pub fn add_dst(&mut self, input_id: SoundInputId, dst_num_states: usize) -> GridSpan {
        let s = StateTableSlice {
            index: self.total_size(),
            count: dst_num_states,
        };
        self.offsets.push((input_id, s));
        GridSpan::new_contiguous(self.total_size(), dst_num_states)
    }

    // Returns the span of states to erase
    pub fn remove_dst(&mut self, input_id: SoundInputId) -> GridSpan {
        let index = self
            .offsets
            .iter()
            .position(|(i, _)| *i == input_id)
            .unwrap();
        let o = self.offsets.remove(index);
        for (_, s) in self.offsets[index..].iter_mut() {
            s.index -= o.1.count;
        }
        GridSpan::new_contiguous(o.1.index, o.1.count)
    }

    // Returns the span of states to insert
    pub fn add_dst_states(&mut self, input_id: SoundInputId, span: GridSpan) -> GridSpan {
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
