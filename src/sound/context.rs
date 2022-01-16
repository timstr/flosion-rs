use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{
    key::Key,
    soundchunk::SoundChunk,
    soundengine::{EngineSoundInputData, EngineSoundProcessorData},
    soundinput::{KeyedSoundInputHandle, SingleSoundInputHandle, SoundInputId},
    soundprocessor::{DynamicSoundProcessorData, SoundProcessorId, StaticSoundProcessorData},
    soundstate::{EmptyState, SoundState},
    statetable::StateTableLock,
};

#[derive(Copy, Clone)]
pub struct SoundProcessorFrame {
    pub id: SoundProcessorId,
    pub state_index: usize,
}

#[derive(Copy, Clone)]
pub struct SingleSoundInputFrame {
    pub id: SoundInputId,
    pub state_index: usize,
}

#[derive(Copy, Clone)]
pub struct KeyedSoundInputFrame {
    pub id: SoundInputId,
    pub key_index: usize,
    pub state_index: usize,
}

#[derive(Copy, Clone)]
pub enum SoundStackFrame {
    Processor(SoundProcessorFrame),
    SingleInput(SingleSoundInputFrame),
    KeyedInput(KeyedSoundInputFrame),
}

impl SoundStackFrame {
    pub fn into_processor_frame(self) -> Option<SoundProcessorFrame> {
        match self {
            SoundStackFrame::Processor(f) => Some(f),
            _ => None,
        }
    }

    pub fn into_single_input_frame(self) -> Option<SingleSoundInputFrame> {
        match self {
            SoundStackFrame::SingleInput(f) => Some(f),
            _ => None,
        }
    }

    pub fn into_keyed_input_frame(self) -> Option<KeyedSoundInputFrame> {
        match self {
            SoundStackFrame::KeyedInput(f) => Some(f),
            _ => None,
        }
    }
}

pub struct Context<'a> {
    processor_data: &'a HashMap<SoundProcessorId, EngineSoundProcessorData>,
    sound_input_data: &'a HashMap<SoundInputId, EngineSoundInputData>,
    stack: Vec<SoundStackFrame>,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        processor_data: &'a HashMap<SoundProcessorId, EngineSoundProcessorData>,
        sound_input_data: &'a HashMap<SoundInputId, EngineSoundInputData>,
        stack: Vec<SoundStackFrame>,
    ) -> Context<'a> {
        assert!(stack.len() > 0);
        Context {
            processor_data,
            sound_input_data,
            stack,
        }
    }

    pub(super) fn single_input_state(
        &self,
        input: &'a SingleSoundInputHandle,
    ) -> StateTableLock<'a, EmptyState> {
        let input_id = input.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::SingleInput(i) => i.id == input_id,
                _ => false,
            })
            .unwrap();
        let f = f.into_single_input_frame().unwrap();
        input.input().get_state(f.state_index)
    }

    pub(super) fn keyed_input_state<K: Key, T: SoundState>(
        &self,
        input: &'a KeyedSoundInputHandle<K, T>,
    ) -> StateTableLock<'a, T> {
        let input_id = input.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::KeyedInput(i) => i.id == input_id,
                _ => false,
            })
            .unwrap();
        let f = f.into_keyed_input_frame().unwrap();
        input.input().get_state(f.state_index, f.key_index)
    }

    pub(super) fn dynamic_sound_processor_state<T: SoundState>(
        &self,
        handle: &'a DynamicSoundProcessorData<T>,
    ) -> StateTableLock<'a, T> {
        let proc_id = handle.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::Processor(i) => i.id == proc_id,
                _ => false,
            })
            .unwrap();
        let f = f.into_processor_frame().unwrap();
        handle.get_state(f.state_index)
    }

    pub(super) fn static_sound_processor_state<T: SoundState>(
        &self,
        handle: &'a StaticSoundProcessorData<T>,
    ) -> &'a RwLock<T> {
        let proc_id = handle.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::Processor(i) => i.id == proc_id,
                _ => false,
            })
            .unwrap();
        let f = f.into_processor_frame().unwrap();
        assert!(f.state_index == 0);
        handle.get_state()
    }

    pub fn current_frame(&self) -> SoundStackFrame {
        *self.stack.last().unwrap()
    }
}

pub struct ProcessorContext<'a, T: SoundState> {
    output_buffer: &'a mut SoundChunk,
    state: &'a RwLock<T>,
    context: Context<'a>,
}

impl<'a, T: SoundState> ProcessorContext<'a, T> {
    pub(super) fn new(
        output_buffer: &'a mut SoundChunk,
        state: &'a RwLock<T>,
        context: Context<'a>,
    ) -> ProcessorContext<'a, T> {
        ProcessorContext {
            output_buffer,
            state,
            context,
        }
    }

    // pub fn context(&self) -> &Context<'a> {
    //     &self.context
    // }

    // pub fn context_mut(&mut self) -> &mut Context<'a> {
    //     &mut self.context
    // }

    pub fn output_buffer(&mut self) -> &mut SoundChunk {
        self.output_buffer
    }

    pub fn step_single_input(&mut self, handle: &SingleSoundInputHandle, dst: &mut SoundChunk) {
        let frame = self.context.stack.last().unwrap();
        let frame = frame.into_processor_frame().unwrap();
        let pd = self.context.processor_data.get(&frame.id).unwrap();
        let input_id = handle.id();
        let input = self.context.sound_input_data.get(&input_id).unwrap();
        assert!(input.input().num_keys() == 1);
        if let Some(target) = input.target() {
            let other_pd = self.context.processor_data.get(&target).unwrap();
            let other_proc = other_pd.sound_processor();
            // TODO: Once I figure things out, make sure that the outputs of previously-invoked
            // static processors are cached, and read the cache right here
            assert!(!other_proc.is_static());
            let mut other_stack = self.context.stack.clone();
            other_stack.push(SoundStackFrame::SingleInput(SingleSoundInputFrame {
                id: input_id,
                state_index: frame.state_index,
            }));
            other_stack.push(SoundStackFrame::Processor(SoundProcessorFrame {
                id: other_proc.id(),
                state_index: other_proc.find_state_index(input_id, frame.state_index),
            }));
            let new_ctx = Context::new(
                self.context.processor_data,
                self.context.sound_input_data,
                other_stack,
            );
            other_proc.process_audio(dst, new_ctx);
        } else {
            dst.silence();
        }
    }

    pub fn step_keyed_input<K: Key, TT: SoundState>(
        &mut self,
        handle: &KeyedSoundInputHandle<K, TT>,
        key_index: usize,
        dst: &mut SoundChunk,
    ) {
        // TODO
        panic!()
    }

    pub fn single_input_state(&self, handle: &SingleSoundInputHandle) {
        // TODO
        panic!()
    }

    pub fn keyed_input_state<K: Key, TT: SoundState>(&self, handle: &KeyedSoundInputHandle<K, TT>) {
        // TODO
        panic!()
    }

    pub fn read_state(&'a self) -> ProcessorStateReadLock<'a, T> {
        ProcessorStateReadLock {
            lock: self.state.read(),
        }
    }

    pub fn write_state<'b>(&'a mut self) -> ProcessorStateWriteLock<'a, 'a, T> {
        ProcessorStateWriteLock {
            lock: self.state.write(),
            _context: &mut self.context,
        }
    }
}

pub struct ProcessorStateReadLock<'a, T: SoundState> {
    lock: RwLockReadGuard<'a, T>,
}

impl<'a, T: SoundState> Deref for ProcessorStateReadLock<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.lock
    }
}

pub struct ProcessorStateWriteLock<'a, 'b, T: SoundState> {
    lock: RwLockWriteGuard<'a, T>,
    // NOTE: storing a mutable reference to the context here is used to ensure
    // that the context is not also used to call upon inputs while a write
    // lock is held. This prevents deadlock.
    _context: &'a mut Context<'b>,
}

impl<'a, 'b, T: SoundState> Deref for ProcessorStateWriteLock<'a, 'b, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.lock
    }
}

impl<'a, 'b, T: SoundState> DerefMut for ProcessorStateWriteLock<'a, 'b, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.lock
    }
}

pub struct NumberContext<'a> {
    context: Context<'a>,
}

impl<'a> NumberContext<'a> {
    pub(super) fn new(context: Context<'a>) -> NumberContext<'a> {
        NumberContext { context }
    }

    pub fn single_input_state(
        &self,
        input: &'a SingleSoundInputHandle,
    ) -> StateTableLock<'a, EmptyState> {
        self.context.single_input_state(input)
    }

    pub fn keyed_input_state<K: Key, T: SoundState>(
        &self,
        input: &'a KeyedSoundInputHandle<K, T>,
    ) -> StateTableLock<'a, T> {
        self.context.keyed_input_state(input)
    }

    pub fn dynamic_sound_processor_state<T: SoundState>(
        &self,
        handle: &'a DynamicSoundProcessorData<T>,
    ) -> StateTableLock<'a, T> {
        self.context.dynamic_sound_processor_state(handle)
    }

    pub fn static_sound_processor_state<T: SoundState>(
        &self,
        handle: &'a StaticSoundProcessorData<T>,
    ) -> &'a RwLock<T> {
        self.context.static_sound_processor_state(handle)
    }
}
