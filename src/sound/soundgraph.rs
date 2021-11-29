use crate::sound::soundchunk::SoundChunk;
use crate::sound::soundinput::InputOptions;
use crate::sound::soundinput::KeyedSoundInput;
use crate::sound::soundinput::SingleSoundInput;
use crate::sound::soundinput::SoundInputId;
use crate::sound::soundprocessor::DynamicSoundProcessor;
use crate::sound::soundprocessor::SoundProcessorId;
use crate::sound::soundprocessor::SoundProcessorWrapper;
use crate::sound::soundprocessor::StaticSoundProcessor;
use crate::sound::soundprocessor::WrappedDynamicSoundProcessor;
use crate::sound::soundprocessor::WrappedStaticSoundProcessor;
use crate::sound::soundstate::{EmptyState, SoundState};
use crate::sound::uniqueid::IdGenerator;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

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
        _input: &KeyedSoundInput<K, T>,
        _key: &K,
    ) -> &mut T {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }

    pub fn state_index(&self) -> usize {
        self.state_index
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
