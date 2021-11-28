use crate::sound::soundchunk::SoundChunk;

use rand::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::iter;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundProcessorId(usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SoundInputId(usize);

impl Default for SoundProcessorId {
    fn default() -> SoundProcessorId {
        SoundProcessorId(1)
    }
}
impl Default for SoundInputId {
    fn default() -> SoundInputId {
        SoundInputId(1)
    }
}

pub struct Context<'a> {
    output_buffer: Option<&'a mut SoundChunk>,
    input_buffers: Vec<(SoundInputId, &'a SoundChunk)>,
    processor_id: SoundProcessorId,
    state_index: usize,
}

impl<'a> Context<'a> {
    fn new(
        output_buffer: Option<&'a mut SoundChunk>,
        input_buffers: Vec<(SoundInputId, &'a SoundChunk)>,
        processor_id: SoundProcessorId,
        state_index: usize,
    ) -> Context<'a> {
        Context {
            output_buffer,
            input_buffers,
            processor_id,
            state_index,
        }
    }

    pub fn has_output(&self) -> bool {
        match self.output_buffer {
            Some(_) => true,
            None => false,
        }
    }

    pub fn output_buffer(&mut self) -> &mut SoundChunk {
        self.output_buffer.as_mut().unwrap()
    }

    pub fn input_buffer(&'a mut self, input_id: SoundInputId) -> &'a SoundChunk {
        // TODO: if the input buffer is not yet filled, call on the sound graph to fill it now
        match self
            .input_buffers
            .iter_mut()
            .find(|(id, _)| *id == input_id)
        {
            Some((_, buffer)) => *buffer,
            None => panic!(),
        }
    }

    pub fn single_input_state(&'a self, _input: &SingleSoundInput) -> &mut EmptyState {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }

    pub fn keyed_input_state<K: Ord, T: SoundState>(
        &'a self,
        _input: &KeyedStateTable<K, T>,
        _key: &K,
    ) -> &mut T {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }
}

#[derive(Copy, Clone)]
pub struct InputOptions {
    // Will the input ever be paused or reset by the sound processor?
    pub interruptible: bool,

    // Will the input's speed of time always be the same as the sound processor's?
    pub realtime: bool,
}

// TODO: data needed for sound inputs:
// single inputs
//     - time offset (at start of chunk?) relative to sound processor
//     - time speed (at start of chunk?) relative to sound processor
//     - no additional per-state data (the sound processor's state will always suffice here)
// keyed inputs:
//     - list of keys (just use usize in the basic interface, map these to custom types somehow when needed)
//     - for each key, the time offset and time speed as above
//     - arbitrary per-key data (e.g. note envelope spline)
//     - arbitrary per-state data (e.g. ensemble note frequency offset)
// for convenience, a single input may be considered to be a keyed input that always has exactly one key

// TODO:
// - Sound processors own their inputs (SingleSoundInput and KeyedSoundInput<K, T>, see below) but they
//   can't do anything with them directly
// - Creating and modifying inputs requires tools from the soundgraph
// - Accessing the (writable) state and (readonly) key data of an input is achieved by
//   passing the strongly-typed (in the case of keyed inputs) input wrapper to the
//   audio processing context
// - the same soundgraph tools needed for modifying the inputs registers the inputs with the
//   sound graph which assumes responsibility for routing and updating states, etc
// - all in all, the sound processor interface is again concerned **only** with DSP calculations,
//   and is simply handed all the pieces it needs by the context object

pub struct SingleSoundInput {
    state_table: StateTable<EmptyState>,
}

pub struct KeyedSoundInput<K: Ord, T: SoundState> {
    state_table: KeyedStateTable<K, T>,
}

#[derive(Copy, Clone)]
pub struct GridSpan {
    // linear index of the first item
    start_index: usize,

    // Number of items in each consecutive group
    items_per_row: usize,

    // Number of items between the start of any two adjacent consecutive groups
    row_stride: usize,

    // Number of consecutive groups
    num_rows: usize,
}

impl GridSpan {
    fn new(
        start_index: usize,
        items_per_row: usize,
        row_stride: usize,
        num_rows: usize,
    ) -> GridSpan {
        assert!(items_per_row > 0);
        assert!(row_stride > 0);
        GridSpan {
            start_index,
            items_per_row,
            row_stride,
            num_rows,
        }
    }

    fn new_contiguous(index: usize, count: usize) -> GridSpan {
        GridSpan::new(index, count, 1, 1)
    }

    fn new_empty() -> GridSpan {
        GridSpan::new(0, 0, 1, 0)
    }

    fn offset(&self, additional_start_offset: usize) -> GridSpan {
        let mut gs = *self;
        gs.start_index += additional_start_offset;
        gs
    }

    fn inflate(&self, items_out_per_items_in: usize) -> GridSpan {
        GridSpan {
            start_index: self.start_index * items_out_per_items_in,
            items_per_row: self.start_index * items_out_per_items_in,
            row_stride: self.row_stride * items_out_per_items_in,
            num_rows: self.num_rows,
        }
    }

    fn contains(&self, index: usize) -> bool {
        if index < self.start_index {
            return false;
        }
        let index = index - self.start_index;
        let inner_index = index % self.row_stride;
        let outer_index = index / self.row_stride;
        (inner_index < self.items_per_row) && (outer_index < self.num_rows)
    }

    fn last_index(&self) -> usize {
        self.start_index + (self.row_stride * self.num_rows) + self.items_per_row - 1
    }

    fn num_items(&self) -> usize {
        self.items_per_row * self.num_rows
    }

    fn insert_with<T, F: Fn() -> T>(&self, data: Vec<T>, f: F) -> Vec<T> {
        if self.num_items() == 0 {
            return data;
        }
        assert!(self.start_index < data.len());
        assert!(self.last_index() < data.len());
        let mut new_states = Vec::<T>::new();
        let old_len = data.len();
        new_states.reserve(old_len + self.num_items());
        for (i, s) in data.into_iter().enumerate() {
            if (i >= self.start_index)
                && (i <= self.last_index())
                && (i - self.start_index) % self.row_stride == 0
            {
                new_states.extend(iter::repeat_with(|| f()).take(self.items_per_row));
            }
            new_states.push(s);
        }
        if self.last_index() + 1 == old_len {
            new_states.extend(iter::repeat_with(|| f()).take(self.items_per_row));
        }
        assert_eq!(new_states.len(), old_len + self.num_items());
        new_states
    }

    fn erase<T>(&self, data: Vec<T>) -> Vec<T> {
        if self.num_items() == 0 {
            return data;
        }
        data.into_iter()
            .enumerate()
            .filter_map(|(i, s)| if self.contains(i) { None } else { Some(s) })
            .collect()
    }
}

pub struct StateTime {
    elapsed_samples: usize,
    relative_time_speed: f32,
}

impl StateTime {
    pub fn new() -> StateTime {
        StateTime {
            elapsed_samples: 0,
            relative_time_speed: 1.0,
        }
    }

    pub fn reset(&mut self) {
        self.elapsed_samples = 0;
        self.relative_time_speed = 1.0;
    }
}

pub trait SoundState: Default {
    fn reset(&mut self);
    fn time(&self) -> &StateTime;
    fn time_mut(&mut self) -> &mut StateTime;
}

pub struct EmptyState {
    time: StateTime,
}

impl Default for EmptyState {
    fn default() -> EmptyState {
        EmptyState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for EmptyState {
    fn reset(&mut self) {}
    fn time(&self) -> &StateTime {
        &self.time
    }
    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}

pub trait DynamicSoundProcessor {
    type StateType: SoundState;
    fn new(sg: &SoundProcessorTools) -> Self;
    fn process_audio(&self, state: &mut Self::StateType, context: &mut Context);
}

pub trait StaticSoundProcessor {
    type StateType: SoundState;
    fn new(sg: &SoundProcessorTools) -> Self;
    fn process_audio(&self, state: &mut Self::StateType, context: &mut Context);
    fn produces_output(&self) -> bool;
}

trait SoundProcessorWrapper {
    // Process the next chunk of audio
    fn process_audio(&self, context: &mut Context);

    // Whether the sound processor is static, e.g. having only one state ever,
    // not allowed to be duplicated, and usually representing an external device
    // such as a speaker or microphone
    fn is_static(&self) -> bool;

    fn num_states(&self) -> usize;

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize;

    // Whether the sound processor produces output, or else just consumes its
    // input buffer for some other purpose
    fn produces_output(&self) -> bool;

    // Allocate states for a newly connected SoundInput
    // Returns the span of states to add to all inputs
    fn add_dst(&mut self, dst_input: SoundInputId, dst_num_states: usize) -> GridSpan;

    // Remove states from a newly detached SoundInput
    // Returns the span of states to remove from all inputs
    fn remove_dst(&mut self, dst_input: SoundInputId) -> GridSpan;

    // Add additional states for a connected SoundInput for upstream
    // states that it has just added
    // Returns the span of states to add to all inputs
    fn insert_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;

    // Remove a subset of states for a connected SoundInput for upstream
    // states that it has just removed
    // Returns the span of states to remove from all inputs
    fn erase_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;

    // Reset a range of states for a connected SoundInput
    // Returns the span of states to reset in all inputs
    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan;
}

struct StateTable<T: SoundState> {
    data: Vec<RefCell<T>>,
}

impl<T: SoundState> StateTable<T> {
    fn new() -> StateTable<T> {
        StateTable { data: Vec::new() }
    }

    fn total_size(&self) -> usize {
        self.data.len()
    }

    fn insert_states(&mut self, span: GridSpan) {
        self.data = span.insert_with(
            std::mem::take(&mut self.data),
            || RefCell::new(T::default()),
        );
    }

    fn erase_states(&mut self, span: GridSpan) {
        self.data = span.erase(std::mem::take(&mut self.data));
    }

    fn reset_states(&self, span: GridSpan) {
        for r in 0..span.num_rows {
            let row_begin = span.start_index + (r * span.row_stride);
            let row_end = row_begin + span.items_per_row;
            for s in &self.data[row_begin..row_end] {
                s.borrow_mut().reset();
            }
        }
    }

    fn get_state<'a>(&'a self, index: usize) -> impl Deref<Target = T> + 'a {
        self.data[index].borrow()
    }

    fn get_state_mut<'a>(&'a self, index: usize) -> impl DerefMut<Target = T> + 'a {
        self.data[index].borrow_mut()
    }
}

struct KeyedStateTable<K: Ord, T: SoundState> {
    keys: Vec<K>,
    data: Vec<RefCell<T>>,
    num_parent_states: usize,
}

impl<K: Ord, T: SoundState> KeyedStateTable<K, T> {
    fn new() -> KeyedStateTable<K, T> {
        KeyedStateTable {
            keys: Vec::new(),
            data: Vec::new(),
            num_parent_states: 0,
        }
    }

    fn add_key(&mut self, key: K) -> GridSpan {
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

    fn remove_key(&mut self, key: K) -> GridSpan {
        let old_num_keys = self.keys.len();
        let index = self.keys.iter().position(|k| *k == key).unwrap();
        self.keys.remove(index);
        let gs = GridSpan::new(index, 1, old_num_keys, self.num_parent_states);
        self.data = gs.erase(std::mem::take(&mut self.data));
        gs
    }

    fn insert_states(&mut self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.keys.len());
        self.data = span.insert_with(
            std::mem::take(&mut self.data),
            || RefCell::new(T::default()),
        );
        span
    }

    fn erase_states(&mut self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.keys.len());
        self.data = span.erase(std::mem::take(&mut self.data));
        span
    }

    fn reset_states(&self, span: GridSpan) -> GridSpan {
        let span = span.inflate(self.keys.len());
        for (i, s) in self.data.iter().enumerate() {
            if span.contains(i) {
                s.borrow_mut().reset();
            }
        }
        span
    }

    fn get_state<'a>(
        &'a self,
        state_index: usize,
        key_index: usize,
    ) -> impl Deref<Target = T> + 'a {
        assert!(key_index < self.keys.len());
        self.data[self.keys.len() * state_index + key_index].borrow()
    }

    fn get_state_mut<'a>(
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

struct StateTablePartition {
    offsets: Vec<(SoundInputId, StateTableSlice)>,
}

impl StateTablePartition {
    fn new() -> StateTablePartition {
        StateTablePartition {
            offsets: Vec::new(),
        }
    }

    fn get_index(&self, input_id: SoundInputId, input_state_index: usize) -> usize {
        assert!(self.offsets.iter().find(|(i, _)| *i == input_id).is_some());
        for (i, s) in self.offsets.iter() {
            if *i == input_id {
                assert!(input_state_index < s.count);
                return s.index + input_state_index;
            }
        }
        panic!();
    }

    fn get_span(&self, input_id: SoundInputId, input_span: GridSpan) -> GridSpan {
        assert!(self.offsets.iter().find(|(i, _)| *i == input_id).is_some());
        for (i, s) in self.offsets.iter() {
            if *i == input_id {
                assert!(input_span.start_index < s.count);
                assert!(input_span.last_index() < s.count);
                return input_span.offset(s.index);
            }
        }
        panic!();
    }

    fn total_size(&self) -> usize {
        let mut acc: usize = 0;
        for (_, s) in self.offsets.iter() {
            assert_eq!(s.index, acc);
            acc += s.count;
        }
        acc
    }

    // Returns the span of states to insert
    fn add_dst(&mut self, input_id: SoundInputId, dst_num_states: usize) -> GridSpan {
        let s = StateTableSlice {
            index: self.total_size(),
            count: dst_num_states,
        };
        self.offsets.push((input_id, s));
        GridSpan::new_contiguous(self.total_size(), dst_num_states)
    }

    // Returns the span of states to erase
    fn remove_dst(&mut self, input_id: SoundInputId) -> GridSpan {
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
    fn add_dst_states(&mut self, input_id: SoundInputId, span: GridSpan) -> GridSpan {
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
    fn remove_dst_states(&mut self, input_id: SoundInputId, span: GridSpan) -> GridSpan {
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

pub struct WrappedDynamicSoundProcessor<T: DynamicSoundProcessor> {
    instance: T,
    id: Option<SoundProcessorId>,
    state_table: StateTable<T::StateType>,
    state_partition: StateTablePartition,
}

impl<T: DynamicSoundProcessor> WrappedDynamicSoundProcessor<T> {
    fn new(instance: T) -> WrappedDynamicSoundProcessor<T> {
        let id = None;
        let state_table = StateTable::new();
        let state_partition = StateTablePartition::new();
        WrappedDynamicSoundProcessor {
            instance,
            id,
            state_table,
            state_partition,
        }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id.unwrap()
    }
}

impl<T: DynamicSoundProcessor> SoundProcessorWrapper for WrappedDynamicSoundProcessor<T> {
    fn process_audio(&self, context: &mut Context) {
        let mut state = self.state_table.get_state_mut(context.state_index);
        self.instance.process_audio(&mut state, context);
    }

    fn is_static(&self) -> bool {
        false
    }

    fn num_states(&self) -> usize {
        assert_eq!(
            self.state_partition.total_size(),
            self.state_table.total_size()
        );
        self.state_table.total_size()
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        self.state_partition.get_index(dst_input, dst_state_index)
    }

    fn produces_output(&self) -> bool {
        true
    }

    fn add_dst(&mut self, dst_input: SoundInputId, dst_num_states: usize) -> GridSpan {
        let s = self.state_partition.add_dst(dst_input, dst_num_states);
        self.state_table.insert_states(s);
        s
    }

    fn remove_dst(&mut self, dst_input: SoundInputId) -> GridSpan {
        let s = self.state_partition.remove_dst(dst_input);
        self.state_table.erase_states(s);
        s
    }

    fn insert_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.add_dst_states(dst_input, span);
        self.state_table.insert_states(s);
        s
    }

    fn erase_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.remove_dst_states(dst_input, span);
        self.state_table.insert_states(s);
        s
    }

    fn reset_states(&self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        let s = self.state_partition.get_span(dst_input, span);
        self.state_table.reset_states(s);
        s
    }
}

struct StaticInputStates {
    input_id: SoundInputId,
    num_states: usize,
}

pub struct WrappedStaticSoundProcessor<T: StaticSoundProcessor> {
    instance: T,
    id: Option<SoundProcessorId>,
    state: RefCell<T::StateType>,
    dst_inputs: Vec<StaticInputStates>,
}

impl<T: StaticSoundProcessor> WrappedStaticSoundProcessor<T> {
    fn new(instance: T) -> WrappedStaticSoundProcessor<T> {
        let id = None;
        let dst_inputs = Vec::new();
        let state = RefCell::new(T::StateType::default());
        WrappedStaticSoundProcessor {
            instance,
            id,
            state,
            dst_inputs,
        }
    }

    pub fn instance(&self) -> &T {
        &self.instance
    }

    pub fn id(&self) -> SoundProcessorId {
        self.id.unwrap()
    }
}

// A static sound processor allows any number of sound inputs to be connected, but all
// will receive copies of the same single audio stream, and all may have at most one
// state.
impl<T: StaticSoundProcessor> SoundProcessorWrapper for WrappedStaticSoundProcessor<T> {
    fn process_audio(&self, context: &mut Context) {
        self.instance
            .process_audio(&mut self.state.borrow_mut(), context);
    }

    fn is_static(&self) -> bool {
        true
    }

    fn num_states(&self) -> usize {
        1
    }

    fn find_state_index(&self, dst_input: SoundInputId, dst_state_index: usize) -> usize {
        assert!(
            match self.dst_inputs.iter().find(|is| is.input_id == dst_input) {
                Some(is) => is.num_states == 1,
                None => false,
            }
        );
        assert_eq!(dst_state_index, 0);
        0
    }

    fn produces_output(&self) -> bool {
        self.instance.produces_output()
    }

    fn add_dst(&mut self, dst_input: SoundInputId, dst_num_states: usize) -> GridSpan {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            0
        );
        if dst_num_states > 1 {
            panic!();
        }
        self.dst_inputs.push(StaticInputStates {
            input_id: dst_input,
            num_states: dst_num_states,
        });
        GridSpan::new_empty()
    }

    fn remove_dst(&mut self, dst_input: SoundInputId) -> GridSpan {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        let i = self
            .dst_inputs
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        self.dst_inputs.remove(i);
        GridSpan::new_empty()
    }

    fn insert_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        if !(span.start_index == 0 && span.num_items() == 1) {
            panic!();
        }
        let i = self
            .dst_inputs
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let si = &mut self.dst_inputs[i];
        if si.num_states == 1 {
            panic!();
        }
        si.num_states = 1;
        GridSpan::new_empty()
    }

    fn erase_dst_states(&mut self, dst_input: SoundInputId, span: GridSpan) -> GridSpan {
        assert!(self.produces_output());
        assert_eq!(
            self.dst_inputs
                .iter()
                .filter(|is| is.input_id == dst_input)
                .count(),
            1
        );
        if !(span.start_index == 0 && span.num_items() == 1) {
            panic!();
        }
        let i = self
            .dst_inputs
            .iter()
            .position(|is| is.input_id == dst_input)
            .unwrap();
        let si = &mut self.dst_inputs[i];
        if si.num_states == 0 {
            panic!();
        }
        si.num_states = 0;
        GridSpan::new_empty()
    }

    fn reset_states(&self, _dst_input: SoundInputId, _span: GridSpan) -> GridSpan {
        // no-op, static sound sources can't be reset
        GridSpan::new_empty()
    }
}

struct SoundProcessorData<'a> {
    wrapper: Rc<RefCell<dyn SoundProcessorWrapper + 'a>>,
}

impl<'a> SoundProcessorData<'a> {
    fn new_dynamic<T: DynamicSoundProcessor + 'a>(
        sg: &SoundProcessorTools,
        _input_idgen: &mut IdGenerator<SoundInputId>,
    ) -> (
        SoundProcessorData<'a>,
        Rc<RefCell<WrappedDynamicSoundProcessor<T>>>,
    ) {
        let w = WrappedDynamicSoundProcessor::<T>::new(T::new(sg));
        let w = Rc::new(RefCell::new(w));
        let w2 = Rc::clone(&w);
        (SoundProcessorData { wrapper: w2 }, w)
    }

    fn new_static<T: StaticSoundProcessor + 'a>(
        sg: &SoundProcessorTools,
        _input_idgen: &mut IdGenerator<SoundInputId>,
    ) -> (
        SoundProcessorData<'a>,
        Rc<RefCell<WrappedStaticSoundProcessor<T>>>,
    ) {
        let w = WrappedStaticSoundProcessor::<T>::new(T::new(sg));
        let w = Rc::new(RefCell::new(w));
        let w2 = Rc::clone(&w);
        (SoundProcessorData { wrapper: w }, w2)
    }

    fn sound_processor(&'a self) -> impl Deref<Target = dyn SoundProcessorWrapper + 'a> {
        self.wrapper.borrow()
    }
}

trait UniqueId: Default + Copy + PartialEq + Eq + Hash {
    fn value(&self) -> usize;
    fn next(&self) -> Self;
}

struct IdGenerator<T: UniqueId> {
    current_id: T,
}

impl<T: UniqueId> IdGenerator<T> {
    fn new() -> IdGenerator<T> {
        IdGenerator {
            current_id: T::default(),
        }
    }

    fn next_id(&mut self) -> T {
        let ret = self.current_id;
        self.current_id = self.current_id.next();
        ret
    }
}

impl UniqueId for SoundProcessorId {
    fn value(&self) -> usize {
        self.0
    }
    fn next(&self) -> SoundProcessorId {
        SoundProcessorId(self.0 + 1)
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

pub struct SoundProcessorDescription {
    is_static: bool,
    inputs: Vec<SoundInputId>,
}

pub struct SoundGraph<'a> {
    processors: HashMap<SoundProcessorId, SoundProcessorData<'a>>,
    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    // TODO: cache routing information
}

pub enum ConnectionError {
    NoChange,
    TooManyConnections,
    CircularDependency,
}

impl<'a> SoundGraph<'a> {
    pub fn new() -> SoundGraph<'a> {
        SoundGraph {
            processors: HashMap::new(),
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
        }
    }

    fn create_processor_tools(&mut self, id: SoundProcessorId) -> SoundProcessorTools {
        // TODO
        panic!()
    }

    pub fn add_dynamic_sound_processor<T: DynamicSoundProcessor + 'a>(
        &mut self,
    ) -> Rc<RefCell<WrappedDynamicSoundProcessor<T>>> {
        let id = self.sound_processor_idgen.next_id();
        let tools = self.create_processor_tools(id);
        let (spdata, sp) =
            SoundProcessorData::new_dynamic::<T>(&tools, &mut self.sound_input_idgen);
        sp.borrow_mut().id = Some(id);
        self.processors.insert(id, spdata);
        sp
    }

    pub fn add_static_sound_processor<T: StaticSoundProcessor + 'a>(
        &mut self,
    ) -> Rc<RefCell<WrappedStaticSoundProcessor<T>>> {
        let id = self.sound_processor_idgen.next_id();
        let tools = self.create_processor_tools(id);
        let (spdata, sp) = SoundProcessorData::new_static::<T>(&tools, &mut self.sound_input_idgen);
        sp.borrow_mut().id = Some(id);
        self.processors.insert(id, spdata);
        sp
    }

    pub fn connect_input(
        &mut self,
        _input_id: SoundInputId,
        _processor: SoundProcessorId,
    ) -> Result<(), ConnectionError> {
        // TODO:
        // allow the new connection unless:
        // - it already exists
        // - it would create a cycle
        // - it would cause a static sound processor to:
        //    - have more than one state per destination input
        //    - be connected to a (directly or transitively) non-realtime input
        // achieve this by creating a lightweight graph description with the same
        // processor and input ids and connections as the current graph, then apply
        // the connection, then test its invariants

        // HACK
        Err(ConnectionError::NoChange)
    }

    pub fn disconnect_input(&mut self, _input_id: SoundInputId) -> Result<(), ConnectionError> {
        // TODO: break any number connections that would be invalidated

        // HACK
        Err(ConnectionError::NoChange)
    }
}

pub struct SoundProcessorTools {
    // TODO
// - id of or ref to the current sound processor
// - reference to any data that might be modified
}

impl SoundProcessorTools {
    pub fn add_single_input(&self, _options: InputOptions) -> SingleSoundInput {
        //TODO
        panic!()
    }

    pub fn add_keyed_input<K: Ord, T: SoundState>(
        &self,
        _options: InputOptions,
    ) -> KeyedSoundInput<K, T> {
        // TODO
        panic!()
    }

    pub fn add_input_key<K: Ord, T: SoundState>(
        &self,
        _input: &mut KeyedSoundInput<K, T>,
        _key: K,
    ) {
        // TODO
        panic!()
    }

    pub fn remove_input_key<K: Ord, T: SoundState>(
        &self,
        _input: &mut KeyedSoundInput<K, T>,
        _key_index: usize,
    ) {
        // TODO
        panic!()
    }

    pub fn num_input_keys<K: Ord, T: SoundState>(&self, _input: &KeyedSoundInput<K, T>) -> usize {
        // TODO
        panic!()
    }

    pub fn get_input_keys<K: Ord, T: SoundState>(&self, _input: &KeyedSoundInput<K, T>) -> Vec<&K> {
        // TODO
        panic!()
    }

    pub fn get_input_keys_mut<K: Ord, T: SoundState>(
        &self,
        _input: &mut KeyedSoundInput<K, T>,
        _key_index: usize,
    ) -> Vec<&mut K> {
        // TODO
        panic!()
    }
}

//---------------------------------------------------------------------------------------------------------------------------

pub struct WhiteNoise {}

pub struct WhiteNoiseState {
    time: StateTime,
}

impl Default for WhiteNoiseState {
    fn default() -> WhiteNoiseState {
        WhiteNoiseState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for WhiteNoiseState {
    fn reset(&mut self) {}
    fn time(&self) -> &StateTime {
        &self.time
    }
    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = WhiteNoiseState;

    fn new(_t: &SoundProcessorTools) -> WhiteNoise {
        WhiteNoise {}
    }

    fn process_audio(&self, _state: &mut WhiteNoiseState, context: &mut Context) {
        let b = context.output_buffer();
        for s in b.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        for s in b.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
    }
}

pub struct DAC {
    input: SingleSoundInput,
    // TODO: stuff for actually playing sound to speakers using CPAL
}

pub struct DACState {
    time: StateTime, // TODO: stuff for actually playing sound to speakers using CPAL
}

impl Default for DACState {
    fn default() -> DACState {
        DACState {
            time: StateTime::new(),
        }
    }
}

impl SoundState for DACState {
    fn reset(&mut self) {}

    fn time(&self) -> &StateTime {
        &self.time
    }

    fn time_mut(&mut self) -> &mut StateTime {
        &mut self.time
    }
}

impl StaticSoundProcessor for DAC {
    type StateType = DACState;

    fn new(t: &SoundProcessorTools) -> DAC {
        DAC {
            input: t.add_single_input(InputOptions {
                realtime: true,
                interruptible: false,
            }),
        }
    }

    fn process_audio(&self, _state: &mut DACState, _context: &mut Context) {
        // TODO
        println!("DAC processing audio");
    }

    fn produces_output(&self) -> bool {
        false
    }
}
