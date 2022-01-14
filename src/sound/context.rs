use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{
    key::Key,
    soundchunk::SoundChunk,
    soundengine::SoundProcessorData,
    soundinput::{KeyedSoundInputHandle, SingleSoundInputHandle},
    soundprocessor::SoundProcessorId,
    soundstate::{EmptyState, SoundState},
};

pub struct Context<'a> {
    output_buffer: Option<&'a mut SoundChunk>,
    processor_data: &'a HashMap<SoundProcessorId, SoundProcessorData>,
    processor_id: SoundProcessorId,
    state_index: usize,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        output_buffer: Option<&'a mut SoundChunk>,
        processor_data: &'a HashMap<SoundProcessorId, SoundProcessorData>,
        processor_id: SoundProcessorId,
        state_index: usize,
    ) -> Context<'a> {
        Context {
            output_buffer,
            processor_data,
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

    pub fn step_single_input(&mut self, handle: &SingleSoundInputHandle, dst: &mut SoundChunk) {
        let pd = self.processor_data.get(&self.processor_id).unwrap();
        let input = pd.inputs().iter().find(|i| i.id() == handle.id()).unwrap();
        assert!(input.input().num_keys() == 1);
        if let Some(target) = input.target() {
            let other_pd = self.processor_data.get(&target).unwrap();
            // TODO: ???
            assert!(!other_pd.sound_processor().is_static());
            let new_ctx = Context::new(Some(dst), self.processor_data, target, self.state_index);
            other_pd.sound_processor().process_audio(new_ctx);
        } else {
            dst.silence();
        }
    }

    pub fn single_input_state(&'a self, input: &SingleSoundInputHandle) -> &mut EmptyState {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }

    pub fn keyed_input_state<K: Key, T: SoundState>(
        &'a self,
        input: &KeyedSoundInputHandle<K, T>,
        key_index: usize,
    ) -> &mut T {
        // TODO: assert that the input belongs to the sound processor
        panic!()
    }

    pub fn state_index(&self) -> usize {
        self.state_index
    }
}

pub struct StateContext<'a, T: SoundState> {
    state: &'a RwLock<T>,
    context: Context<'a>,
}

impl<'a, T: SoundState> StateContext<'a, T> {
    pub fn new(state: &'a RwLock<T>, context: Context<'a>) -> StateContext<'a, T> {
        StateContext { state, context }
    }

    pub fn context(&self) -> &Context<'a> {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut Context<'a> {
        &mut self.context
    }

    pub fn read_state(&'a self) -> StateReadLock<'a, T> {
        StateReadLock {
            lock: self.state.read(),
        }
    }

    pub fn write_state<'b>(&'a mut self) -> StateWriteLock<'a, 'a, T> {
        StateWriteLock {
            lock: self.state.write(),
            _context: &mut self.context,
        }
    }
}

pub struct StateReadLock<'a, T: SoundState> {
    lock: RwLockReadGuard<'a, T>,
}

impl<'a, T: SoundState> Deref for StateReadLock<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.lock
    }
}

pub struct StateWriteLock<'a, 'b, T: SoundState> {
    lock: RwLockWriteGuard<'a, T>,
    // NOTE: storing a mutable reference to the context here is used to ensure
    // that the context is not also used to call upon inputs while a write
    // lock is held. This prevents deadlock.
    _context: &'a mut Context<'b>,
}

impl<'a, 'b, T: SoundState> Deref for StateWriteLock<'a, 'b, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.lock
    }
}

impl<'a, 'b, T: SoundState> DerefMut for StateWriteLock<'a, 'b, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.lock
    }
}
