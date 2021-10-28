use crate::sound::soundchunk::SoundChunk;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;

#[derive(Copy, Clone)]
pub struct SoundProcessorId(usize);

#[derive(Copy, Clone)]
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
    // TODO
}

pub trait SoundState: Default {
    fn reset(&mut self);
}

pub trait SoundProcessor: Default {
    type StateType: SoundState;
    fn get_next_chunk(&self, state: &mut Self::StateType, context: &mut Context);
    fn get_num_inputs(&self) -> usize;
}

trait SoundProcessorWrapper {
    fn get_next_chunk(&self, context: &mut Context);
    fn get_num_inputs(&self) -> usize;
}

struct WrappedSoundProcessor<T: SoundProcessor> {
    instance: T,
    // TODO: state table
    // HACK: just one state
    state: RefCell<<T as SoundProcessor>::StateType>,
}

impl<T: SoundProcessor> WrappedSoundProcessor<T> {
    fn new() -> WrappedSoundProcessor<T> {
        let instance = T::default();
        let state = RefCell::new(T::StateType::default());
        WrappedSoundProcessor { instance, state }
    }
}

impl<T: SoundProcessor> SoundProcessorWrapper for WrappedSoundProcessor<T> {
    fn get_next_chunk(&self, context: &mut Context) {
        self.instance
            .get_next_chunk(&mut self.state.borrow_mut(), context);
    }
    fn get_num_inputs(&self) -> usize {
        self.instance.get_num_inputs()
    }
}

struct SoundProcessorData {
    wrapper: Box<dyn SoundProcessorWrapper>,
    inputs: Vec<(SoundInputId, Option<SoundProcessorId>)>,
}

impl SoundProcessorData {
    fn new<T: SoundProcessor>(input_idgen: &mut IdGenerator<SoundInputId>) -> SoundProcessorData {
        let wrapper: Box<WrappedSoundProcessor<T>> = Box::new(WrappedSoundProcessor::<T>::new());
        let inputs: Vec<_> = (0..wrapper.get_num_inputs())
            .map(|_| (input_idgen.next_id(), None))
            .collect();
        let wrapper: Box<dyn SoundProcessorWrapper> = wrapper;
        SoundProcessorData { wrapper, inputs }
    }

    fn sound_processor<'a>(&'a self) -> impl Deref<Target = dyn SoundProcessorWrapper + 'a> {
        self.wrapper.deref()
    }
}

trait UniqueId: Default + Copy {
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

pub struct SoundGraph {
    processors: HashMap<SoundProcessorId, SoundProcessorData>,
    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        SoundGraph {
            processors: HashMap::new(),
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
        }
    }

    pub fn add_sound_processor<T: SoundProcessor>() -> SoundProcessorId {}
}
