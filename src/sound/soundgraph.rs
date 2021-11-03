use crate::sound::soundchunk::SoundChunk;

use rand::prelude::*;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

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
    in_out_buffer: &'a mut SoundChunk,
    other_input_buffers: Vec<(SoundInputId, &'a mut SoundChunk)>,
    dst: SoundInputId,
    dst_state_index: usize,
}

impl<'a> Context<'a> {
    fn new(
        in_out_buffer: &'a mut SoundChunk,
        other_input_buffers: Vec<(SoundInputId, &'a mut SoundChunk)>,
        dst: SoundInputId,
        dst_state_index: usize,
    ) -> Context<'a> {
        Context {
            in_out_buffer,
            other_input_buffers,
            dst,
            dst_state_index,
        }
    }

    pub fn in_out_buffer(&mut self) -> &mut SoundChunk {
        self.in_out_buffer
    }

    pub fn input_buffer(&'a mut self, input_id: SoundInputId) -> &'a mut SoundChunk {
        match self
            .other_input_buffers
            .iter_mut()
            .find(|(id, _)| *id == input_id)
        {
            Some((_, buffer)) => *buffer,
            None => panic!(),
        }
    }

    pub fn dst(&self) -> SoundInputId {
        self.dst
    }

    pub fn dst_state_index(&self) -> usize {
        self.dst_state_index
    }
}

pub trait SoundState: Default {
    fn reset(&mut self);
}

pub trait DynamicSoundProcessor {
    type StateType: SoundState;
    fn process_audio(&self, state: &mut Self::StateType, context: &mut Context);
    fn get_num_inputs(&self) -> usize;
}
pub trait StaticSoundProcessor {
    fn process_audio(&mut self, context: &mut Context);
    fn get_num_inputs(&self) -> usize;
    fn reset(&mut self);
}

trait SoundProcessorWrapper {
    fn process_audio(&self, context: &mut Context);
    fn get_num_inputs(&self) -> usize;
    fn is_static(&self) -> bool;
    fn produces_output(&self) -> bool;
    fn add_dst(&mut self, dst_input: SoundInputId, dst_num_states: usize);
    fn remove_dst(&mut self, dst_input: SoundInputId);
    fn insert_dst_states(&mut self, dst_input: SoundInputId, start_index: usize, num: usize);
    fn erase_dst_states(&mut self, dst_input: SoundInputId, start_index: usize, num: usize);
    fn reset_state(&self, dst_input: SoundInputId, index: usize);
}

struct StateTable<T: SoundState> {
    data: Vec<RefCell<T>>,
    offsets: Vec<(SoundInputId, usize)>,
}

impl<T: SoundState> StateTable<T> {
    fn new() -> StateTable<T> {
        StateTable {
            data: Vec::new(),
            offsets: Vec::new(),
        }
    }

    fn get_index(&self, input_id: SoundInputId, input_state_index: usize) -> usize {
        let mut index = input_state_index;
        assert_ne!(self.offsets.iter().find(|(i, _)| *i == input_id), None);
        for (i, o) in self.offsets.iter() {
            index += o;
            if *i == input_id {
                break;
            }
        }
        assert!(index <= self.data.len());
        index
    }

    fn get_state_mut<'a>(
        &'a self,
        input_id: SoundInputId,
        input_state_index: usize,
    ) -> impl DerefMut<Target = T> + 'a {
        let i = self.get_index(input_id, input_state_index);
        self.data[i].borrow_mut()
    }
}

struct WrappedDynamicSoundProcessor<T: DynamicSoundProcessor> {
    instance: T,
    state_table: StateTable<T::StateType>,
}

impl<T: DynamicSoundProcessor> WrappedDynamicSoundProcessor<T> {
    fn new(instance: T) -> WrappedDynamicSoundProcessor<T> {
        let state_table = StateTable::new();
        WrappedDynamicSoundProcessor {
            instance,
            state_table,
        }
    }
}

impl<T: DynamicSoundProcessor> SoundProcessorWrapper for WrappedDynamicSoundProcessor<T> {
    fn process_audio(&self, context: &mut Context) {
        let mut state = self
            .state_table
            .get_state_mut(context.dst(), context.dst_state_index());
        self.instance.process_audio(&mut state, context);
    }

    fn get_num_inputs(&self) -> usize {
        self.instance.get_num_inputs()
    }

    fn is_static(&self) -> bool {
        false
    }
}

struct WrappedStaticSoundProcessor<T: StaticSoundProcessor> {
    instance: RefCell<T>,
}

impl<T: StaticSoundProcessor> WrappedStaticSoundProcessor<T> {
    fn new(instance: T) -> WrappedStaticSoundProcessor<T> {
        let instance = RefCell::new(instance);
        WrappedStaticSoundProcessor { instance }
    }
}

impl<T: StaticSoundProcessor> SoundProcessorWrapper for WrappedStaticSoundProcessor<T> {
    fn process_audio(&self, context: &mut Context) {
        self.instance.borrow_mut().process_audio(context);
    }

    fn get_num_inputs(&self) -> usize {
        self.instance.borrow().get_num_inputs()
    }

    fn is_static(&self) -> bool {
        true
    }
}

struct SoundProcessorData<'a> {
    wrapper: Box<dyn SoundProcessorWrapper + 'a>,
    inputs: Vec<(SoundInputId, Option<SoundProcessorId>)>,
}

impl<'a> SoundProcessorData<'a> {
    fn new_dynamic<T: DynamicSoundProcessor + 'a>(
        sound_processor: T,
        input_idgen: &mut IdGenerator<SoundInputId>,
    ) -> SoundProcessorData<'a> {
        let wrapper = Box::new(WrappedDynamicSoundProcessor::<T>::new(sound_processor));
        let inputs = SoundProcessorData::create_inputs_for(&*wrapper, input_idgen);
        SoundProcessorData { wrapper, inputs }
    }

    fn new_static<T: StaticSoundProcessor + 'a>(
        sound_processor: T,
        input_idgen: &mut IdGenerator<SoundInputId>,
    ) -> SoundProcessorData<'a> {
        let wrapper = Box::new(WrappedStaticSoundProcessor::<T>::new(sound_processor));
        let inputs = SoundProcessorData::create_inputs_for(&*wrapper, input_idgen);
        SoundProcessorData { wrapper, inputs }
    }

    fn sound_processor(&'a self) -> impl Deref<Target = dyn SoundProcessorWrapper + 'a> {
        self.wrapper.deref()
    }

    fn create_inputs_for(
        wrapper: &dyn SoundProcessorWrapper,
        input_idgen: &mut IdGenerator<SoundInputId>,
    ) -> Vec<(SoundInputId, Option<SoundProcessorId>)> {
        (0..wrapper.get_num_inputs())
            .map(|_| (input_idgen.next_id(), None))
            .collect()
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

pub struct SoundGraph<'a> {
    processors: HashMap<SoundProcessorId, SoundProcessorData<'a>>,
    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    // TODO: cache routing information
}

enum ConnectionError {
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

    pub fn add_dynamic_sound_processor<T: DynamicSoundProcessor + 'a>(
        &mut self,
        sound_processor: T,
    ) -> SoundProcessorId {
        let id = self.sound_processor_idgen.next_id();
        let spdata =
            SoundProcessorData::new_dynamic::<T>(sound_processor, &mut self.sound_input_idgen);
        self.processors.insert(id, spdata);
        id
    }

    pub fn add_static_sound_processor<T: StaticSoundProcessor + 'a>(
        &mut self,
        sound_processor: T,
    ) -> SoundProcessorId {
        let id = self.sound_processor_idgen.next_id();
        let spdata =
            SoundProcessorData::new_static::<T>(sound_processor, &mut self.sound_input_idgen);
        self.processors.insert(id, spdata);
        id
    }

    pub fn connect_input(
        &mut self,
        input_id: SoundInputId,
        processor: SoundProcessorId,
    ) -> Result<(), ConnectionError> {
        // TODO
    }

    pub fn disconnect_input(&mut self, input_id: SoundInputId) -> Result<(), ConnectionError> {
        // TODO
    }
}

pub struct WhiteNoise {}

pub struct WhiteNoiseState {}

impl Default for WhiteNoiseState {
    fn default() -> WhiteNoiseState {
        WhiteNoiseState {}
    }
}

impl SoundState for WhiteNoiseState {
    fn reset(&mut self) {}
}

impl DynamicSoundProcessor for WhiteNoise {
    type StateType = WhiteNoiseState;
    fn process_audio(&self, _state: &mut WhiteNoiseState, context: &mut Context) {
        let b = context.in_out_buffer();
        for s in b.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
        for s in b.l.iter_mut() {
            let r: f32 = thread_rng().gen();
            *s = 0.2 * r - 0.1;
        }
    }
    fn get_num_inputs(&self) -> usize {
        0
    }
}

pub struct DAC {
    // TODO
}

impl StaticSoundProcessor for DAC {
    fn process_audio(&mut self, context: &mut Context) {}
    fn get_num_inputs(&self) -> usize {
        1
    }
    fn reset(&mut self) {}
}
