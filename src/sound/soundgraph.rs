use crate::sound::soundchunk::SoundChunk;
use crate::sound::soundinput::InputOptions;
use crate::sound::soundinput::KeyedSoundInput;
use crate::sound::soundinput::KeyedSoundInputHandle;
use crate::sound::soundinput::SingleSoundInput;
use crate::sound::soundinput::SingleSoundInputHandle;
use crate::sound::soundinput::SoundInputId;
use crate::sound::soundinput::SoundInputWrapper;
use crate::sound::soundinput::WrappedKeyedSoundInput;
use crate::sound::soundinput::WrappedSingleSoundInput;
use crate::sound::soundprocessor::DynamicSoundProcessor;
use crate::sound::soundprocessor::SoundProcessorId;
use crate::sound::soundprocessor::SoundProcessorWrapper;
use crate::sound::soundprocessor::StaticSoundProcessor;
use crate::sound::soundprocessor::WrappedDynamicSoundProcessor;
use crate::sound::soundprocessor::WrappedStaticSoundProcessor;
use crate::sound::soundstate::{EmptyState, SoundState};
use crate::sound::uniqueid::IdGenerator;
use std::ops::DerefMut;

use std::cell::{Ref, RefCell, RefMut};
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

struct SoundInputData<'a> {
    id: SoundInputId,
    input: Rc<RefCell<dyn 'a + SoundInputWrapper>>,
    target: Option<SoundProcessorId>,
    options: InputOptions,
}

struct SoundProcessorData<'a> {
    id: SoundProcessorId,
    wrapper: Rc<RefCell<dyn 'a + SoundProcessorWrapper>>,
    inputs: Vec<SoundInputData<'a>>,
}

pub struct DynamicSoundProcessorHandle<T: DynamicSoundProcessor> {
    instance: Rc<RefCell<WrappedDynamicSoundProcessor<T>>>,
}

impl<T: DynamicSoundProcessor> DynamicSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.instance.borrow().id()
    }

    pub fn instance(&self) -> DynamicSoundProcessorRef<T> {
        DynamicSoundProcessorRef {
            cell_ref: self.instance.borrow(),
        }
    }

    pub fn instance_mut(&self) -> DynamicSoundProcessorRefMut<T> {
        DynamicSoundProcessorRefMut {
            cell_ref: self.instance.borrow_mut(),
        }
    }
}

pub struct DynamicSoundProcessorRef<'a, T: DynamicSoundProcessor> {
    cell_ref: Ref<'a, WrappedDynamicSoundProcessor<T>>,
}

pub struct DynamicSoundProcessorRefMut<'a, T: DynamicSoundProcessor> {
    cell_ref: RefMut<'a, WrappedDynamicSoundProcessor<T>>,
}

impl<'a, T: DynamicSoundProcessor> Deref for DynamicSoundProcessorRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.cell_ref.instance()
    }
}

impl<'a, T: DynamicSoundProcessor> Deref for DynamicSoundProcessorRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.cell_ref.instance()
    }
}

impl<'a, T: DynamicSoundProcessor> DerefMut for DynamicSoundProcessorRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.cell_ref.instance_mut()
    }
}

pub struct StaticSoundProcessorHandle<T: StaticSoundProcessor> {
    instance: Rc<RefCell<WrappedStaticSoundProcessor<T>>>,
}

impl<T: StaticSoundProcessor> StaticSoundProcessorHandle<T> {
    pub fn id(&self) -> SoundProcessorId {
        self.instance.borrow().id()
    }

    pub fn instance(&self) -> StaticSoundProcessorRef<T> {
        StaticSoundProcessorRef {
            cell_ref: self.instance.borrow(),
        }
    }

    pub fn instance_mut(&mut self) -> StaticSoundProcessorRefMut<T> {
        StaticSoundProcessorRefMut {
            cell_ref: self.instance.borrow_mut(),
        }
    }
}

pub struct StaticSoundProcessorRef<'a, T: StaticSoundProcessor> {
    cell_ref: Ref<'a, WrappedStaticSoundProcessor<T>>,
}

pub struct StaticSoundProcessorRefMut<'a, T: StaticSoundProcessor> {
    cell_ref: RefMut<'a, WrappedStaticSoundProcessor<T>>,
}

impl<'a, T: StaticSoundProcessor> Deref for StaticSoundProcessorRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.cell_ref.instance()
    }
}

impl<'a, T: StaticSoundProcessor> Deref for StaticSoundProcessorRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.cell_ref.instance()
    }
}

impl<'a, T: StaticSoundProcessor> DerefMut for StaticSoundProcessorRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.cell_ref.instance_mut()
    }
}

impl<'a> SoundProcessorData<'a> {
    fn new_dynamic<'b, T: 'a + DynamicSoundProcessor>(
        proc_idgen: &'b mut IdGenerator<SoundProcessorId>,
        input_idgen: &'b mut IdGenerator<SoundInputId>,
    ) -> (SoundProcessorData<'a>, DynamicSoundProcessorHandle<T>) {
        let id = proc_idgen.next_id();
        let mut inputs = Vec::<SoundInputData>::new();
        let w1;
        {
            let tools = SoundProcessorTools {
                input_idgen,
                inputs: &mut inputs,
            };
            let w = WrappedDynamicSoundProcessor::<T>::new(T::new(tools));
            w1 = Rc::new(RefCell::new(w));
        }
        let w2 = Rc::clone(&w1);
        (
            SoundProcessorData {
                id,
                wrapper: w2,
                inputs,
            },
            DynamicSoundProcessorHandle { instance: w1 },
        )
    }

    fn new_static<'b, T: 'a + StaticSoundProcessor>(
        proc_idgen: &'b mut IdGenerator<SoundProcessorId>,
        input_idgen: &'b mut IdGenerator<SoundInputId>,
    ) -> (SoundProcessorData<'a>, StaticSoundProcessorHandle<T>) {
        let id = proc_idgen.next_id();
        let mut inputs = Vec::<SoundInputData>::new();
        let w1;
        {
            let tools = SoundProcessorTools {
                input_idgen,
                inputs: &mut inputs,
            };
            let w = WrappedStaticSoundProcessor::<T>::new(T::new(tools));
            w1 = Rc::new(RefCell::new(w));
        }
        let w2 = Rc::clone(&w1);
        (
            SoundProcessorData {
                id,
                wrapper: w2,
                inputs,
            },
            StaticSoundProcessorHandle { instance: w1 },
        )
    }

    fn sound_processor(&'a self) -> impl Deref<Target = dyn 'a + SoundProcessorWrapper> {
        self.wrapper.borrow()
    }
}

#[derive(Debug)]
pub enum ConnectionError {
    NoChange,
    CircularDependency,
    TooManyStates,
    ProcessorNotFound,
    InputNotFound,
    InputOccupied,
}

struct SoundInputDescription {
    id: SoundInputId,
    options: InputOptions,
    num_keys: usize,
    target: Option<SoundProcessorId>,
}

struct SoundProcessorDescription {
    id: SoundProcessorId,
    is_static: bool,
    inputs: Vec<SoundInputDescription>,
}

struct SoundGraphDescription {
    processors: Vec<SoundProcessorDescription>,
}

impl SoundGraphDescription {
    fn new(processors: Vec<SoundProcessorDescription>) -> SoundGraphDescription {
        SoundGraphDescription { processors }
    }

    fn find_error(&self) -> Option<ConnectionError> {
        if self.contains_cycles() {
            return Some(ConnectionError::CircularDependency);
        }
        if self.too_many_states() {
            return Some(ConnectionError::TooManyStates);
        }
        None
    }

    fn add_connection(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Option<ConnectionError> {
        if self
            .processors
            .iter()
            .find(|p| p.id == processor_id)
            .is_none()
        {
            return Some(ConnectionError::ProcessorNotFound);
        }
        for p in self.processors.iter() {
            let i = match p.inputs.iter().find(|i| i.id == input_id) {
                None => continue,
                Some(i) => i,
            };
            if let Some(prev_proc) = i.target {
                if prev_proc == processor_id {
                    return Some(ConnectionError::NoChange);
                }
                return Some(ConnectionError::InputOccupied);
            }
            return None;
        }
        Some(ConnectionError::InputNotFound)
    }

    fn contains_cycles(&self) -> bool {
        fn dfs_find_cycle(
            id: SoundProcessorId,
            visited: &mut Vec<SoundProcessorId>,
            path: &mut Vec<SoundProcessorId>,
            processors: &Vec<SoundProcessorDescription>,
        ) -> bool {
            // If the current path already contains this processor, there is a cycle
            if path.contains(&id) {
                return true;
            }
            if !visited.contains(&id) {
                visited.push(id)
            }
            path.push(id);
            let mut found_cycle = false;
            let p = processors.iter().find(|spd| spd.id == id).unwrap();
            for i in p.inputs.iter().filter_map(|input| input.target) {
                if dfs_find_cycle(i, visited, path, processors) {
                    found_cycle = true;
                    break;
                }
            }
            assert_eq!(path[path.len() - 1], id);
            path.pop();
            found_cycle
        }
        let mut visited: Vec<SoundProcessorId> = vec![];
        let mut path: Vec<SoundProcessorId> = vec![];
        loop {
            assert_eq!(visited.len(), 0);
            assert_eq!(path.len(), 0);
            match self.processors.iter().find(|p| !visited.contains(&p.id)) {
                None => break false,
                Some(i) => {
                    if dfs_find_cycle(i.id, &mut visited, &mut path, &self.processors) {
                        break true;
                    }
                }
            }
        }
    }

    fn too_many_states(&self) -> bool {
        // TODO:
        // - count the number of states at each sound processor
        //   - assign 0 to all processors
        //   - dfs starting at all static processors, incrementing states and
        //     multiplying current number by input keys when visiting. do not
        //     traverse through other static processors
        //   - if a static sound processor is visited and the total number of
        //     states along the current path is more than 1, error.
        panic!()
    }
}

pub struct SoundGraph<'a> {
    processors: HashMap<SoundProcessorId, SoundProcessorData<'a>>,
    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    // TODO: cache routing information
}

impl<'a> SoundGraph<'a> {
    pub fn new() -> SoundGraph<'a> {
        SoundGraph {
            processors: HashMap::new(),
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
        }
    }

    pub fn add_dynamic_sound_processor<'b, T: 'a + DynamicSoundProcessor>(
        &'b mut self,
    ) -> DynamicSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let (spdata, sp) = SoundProcessorData::new_dynamic::<T>(
            &mut self.sound_processor_idgen,
            &mut self.sound_input_idgen,
        );
        sp.instance.borrow_mut().id = Some(id);
        self.processors.insert(id, spdata);
        sp
    }

    pub fn add_static_sound_processor<'b, T: 'a + StaticSoundProcessor>(
        &'b mut self,
    ) -> StaticSoundProcessorHandle<T> {
        let id = self.sound_processor_idgen.next_id();
        let (spdata, sp) = SoundProcessorData::new_static::<T>(
            &mut self.sound_processor_idgen,
            &mut self.sound_input_idgen,
        );
        sp.instance.borrow_mut().id = Some(id);
        self.processors.insert(id, spdata);
        sp
    }

    pub fn connect_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), ConnectionError> {
        let mut desc = self.describe();
        assert!(desc.find_error().is_none());

        if let Some(err) = desc.add_connection(input_id, processor_id) {
            return Err(err);
        }

        if let Some(err) = desc.find_error() {
            return Err(err);
        }

        for (proc_id, proc) in self.processors.iter() {
            let i = proc.inputs.iter().find(|i| i.id == input_id);
            // TODO
        }

        // TODO
        Ok(())
    }

    pub fn disconnect_input(&mut self, _input_id: SoundInputId) -> Result<(), ConnectionError> {
        // TODO
        panic!()
    }

    fn describe(&self) -> SoundGraphDescription {
        let mut processors = Vec::<SoundProcessorDescription>::new();
        for (proc_id, proc) in self.processors.iter() {
            let mut inputs = Vec::<SoundInputDescription>::new();
            for i in proc.inputs.iter() {
                let input_instance = i.input.borrow();
                inputs.push(SoundInputDescription {
                    id: input_instance.id(),
                    num_keys: input_instance.num_keys(),
                    options: i.options,
                    target: i.target,
                })
            }
            processors.push(SoundProcessorDescription {
                id: *proc_id,
                inputs,
                is_static: proc.wrapper.borrow().is_static(),
            })
        }
        SoundGraphDescription { processors }
    }
}

pub struct SoundProcessorTools<'a, 'b> {
    input_idgen: &'a mut IdGenerator<SoundInputId>,
    inputs: &'a mut Vec<SoundInputData<'b>>,
    // TODO
    // - id of or ref to the current sound processor
    // - reference to any data that might be modified
}

impl<'a, 'b> SoundProcessorTools<'a, 'b> {
    pub fn add_single_input(&mut self, options: InputOptions) -> SingleSoundInputHandle {
        let input = WrappedSingleSoundInput::new(self.input_idgen);
        let input = Rc::new(RefCell::new(input));
        let input2 = Rc::clone(&input);
        self.inputs.push(SoundInputData {
            id: input.borrow().id(),
            input: input2,
            options,
            target: None,
        });
        SingleSoundInputHandle::new(input)
    }

    pub fn add_keyed_input<K: 'b + Ord, T: 'b + SoundState>(
        &mut self,
        options: InputOptions,
    ) -> KeyedSoundInputHandle<K, T> {
        let input = WrappedKeyedSoundInput::<K, T>::new(self.input_idgen);
        let input = Rc::new(RefCell::new(input));
        let input2 = Rc::clone(&input);
        self.inputs.push(SoundInputData {
            id: input.borrow().id(),
            input: input2,
            options,
            target: None,
        });
        KeyedSoundInputHandle::new(input)
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
