use std::collections::HashMap;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::soundinput::SoundInputWrapper;

use super::{
    gridspan::GridSpan,
    key::Key,
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    numeric,
    scratcharena::{ScratchArena, ScratchSlice},
    soundchunk::SoundChunk,
    soundengine::{
        EngineNumberInputData, EngineNumberSourceData, EngineSoundInputData,
        EngineSoundProcessorData,
    },
    soundinput::{KeyedSoundInputHandle, SingleSoundInputHandle, SoundInputId},
    soundprocessor::{SoundProcessorData, SoundProcessorId},
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
    sound_processor_data: &'a HashMap<SoundProcessorId, EngineSoundProcessorData>,
    sound_input_data: &'a HashMap<SoundInputId, EngineSoundInputData>,
    number_source_data: &'a HashMap<NumberSourceId, EngineNumberSourceData>,
    number_input_data: &'a HashMap<NumberInputId, EngineNumberInputData>,
    static_processor_cache: &'a Vec<(SoundProcessorId, Option<SoundChunk>)>,
    stack: Vec<SoundStackFrame>,
    scratch_space: &'a ScratchArena,
}

impl<'a> Context<'a> {
    pub(super) fn new(
        processor_data: &'a HashMap<SoundProcessorId, EngineSoundProcessorData>,
        sound_input_data: &'a HashMap<SoundInputId, EngineSoundInputData>,
        number_source_data: &'a HashMap<NumberSourceId, EngineNumberSourceData>,
        number_input_data: &'a HashMap<NumberInputId, EngineNumberInputData>,
        static_processor_cache: &'a Vec<(SoundProcessorId, Option<SoundChunk>)>,
        stack: Vec<SoundStackFrame>,
        scratch_space: &'a ScratchArena,
    ) -> Context<'a> {
        debug_assert!(
            stack.len() > 0,
            "Attempted to create a Context object with an empty sound frame stack"
        );
        Context {
            sound_processor_data: processor_data,
            sound_input_data,
            number_source_data,
            number_input_data,
            static_processor_cache,
            stack,
            scratch_space,
        }
    }

    pub(super) fn single_input_state_from_context(
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

    pub(super) fn reset_single_input(&self, handle: &SingleSoundInputHandle) {
        debug_assert!(
            handle.input().options().interruptible,
            "Attempted to reset an uninterruptible SingleSoundInput"
        );
        let input_data = self.sound_input_data.get(&handle.id()).unwrap();
        let state_index = self
            .current_frame()
            .into_processor_frame()
            .unwrap()
            .state_index;
        let gs = handle.input().reset_key(state_index, 0);
        if let Some(spid) = input_data.target() {
            self.reset_recursively(spid, gs, handle.id());
        }
    }

    pub(super) fn keyed_input_state<K: Key, T: SoundState>(
        &self,
        input: &'a KeyedSoundInputHandle<K, T>,
        state_index: usize,
        key_index: usize,
    ) -> StateTableLock<'a, T> {
        input.input().get_state(state_index, key_index)
    }

    pub(super) fn keyed_input_state_from_context<K: Key, T: SoundState>(
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

    pub(super) fn sound_processor_state_from_context<T: SoundState>(
        &self,
        handle: &'a SoundProcessorData<T>,
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

    pub(super) fn reset_keyed_input<K: Key, TT: SoundState>(
        &self,
        handle: &KeyedSoundInputHandle<K, TT>,
        key_index: usize,
    ) {
        debug_assert!(
            handle.input().options().interruptible,
            "Attempted to reset an uninterruptible SoundInput"
        );
        let input_data = self.sound_input_data.get(&handle.id()).unwrap();
        let state_index = self
            .current_frame()
            .into_processor_frame()
            .unwrap()
            .state_index;
        let gs = handle.input().reset_key(state_index, key_index);
        if let Some(spid) = input_data.target() {
            self.reset_recursively(spid, gs, handle.id());
        }
    }

    pub(super) fn evaluate_number_input(&self, input_id: NumberInputId, dst: &mut [f32]) {
        let input_data = self.number_input_data.get(&input_id).unwrap();
        match input_data.target() {
            Some(nsid) => {
                let source_data = self.number_source_data.get(&nsid).unwrap();
                source_data.instance().eval(dst, NumberContext::new(self));
            }
            None => numeric::fill(dst, 0.0),
        }
    }

    pub fn current_frame(&self) -> SoundStackFrame {
        *self.stack.last().unwrap()
    }

    fn step_sound_input(
        &mut self,
        input_id: SoundInputId,
        key_index: Option<usize>,
        dst: &mut SoundChunk,
    ) {
        let frame = self.stack.last().unwrap();
        let frame = frame.into_processor_frame().unwrap();
        let input = self.sound_input_data.get(&input_id).unwrap();
        if let Some(target) = input.target() {
            let other_pd = self.sound_processor_data.get(&target).unwrap();
            let other_proc = other_pd.wrapper();
            if other_proc.is_static() {
                let ch = self.get_cached_static_output(other_pd.id()).unwrap();
                dst.copy_from(ch);
            } else {
                let mut other_stack = self.stack.clone();
                let key_index = if let Some(k_idx) = key_index {
                    other_stack.push(SoundStackFrame::KeyedInput(KeyedSoundInputFrame {
                        id: input_id,
                        state_index: frame.state_index,
                        key_index: k_idx,
                    }));
                    k_idx
                } else {
                    other_stack.push(SoundStackFrame::SingleInput(SingleSoundInputFrame {
                        id: input_id,
                        state_index: frame.state_index,
                    }));
                    0
                };
                let input_state_index = frame.state_index * input.input().num_keys() + key_index;
                let state_index = other_proc.find_state_index(input_id, input_state_index);
                other_stack.push(SoundStackFrame::Processor(SoundProcessorFrame {
                    id: other_proc.id(),
                    state_index,
                }));
                let new_ctx = Context::new(
                    self.sound_processor_data,
                    self.sound_input_data,
                    self.number_source_data,
                    self.number_input_data,
                    self.static_processor_cache,
                    other_stack,
                    self.scratch_space,
                );
                other_proc.process_audio(dst, new_ctx);
            }
        } else {
            dst.silence();
        }
    }

    fn reset_recursively(
        &self,
        processor_id: SoundProcessorId,
        grid_span: GridSpan,
        dst_input: SoundInputId,
    ) {
        let proc_data = self.sound_processor_data.get(&processor_id).unwrap();
        let gs = proc_data.wrapper().reset_states(dst_input, grid_span);
        for input_id in proc_data.inputs().iter() {
            let input_data = self.sound_input_data.get(input_id).unwrap();
            let gs = input_data.input().reset_states(gs);
            if let Some(spid) = input_data.target() {
                self.reset_recursively(spid, gs, *input_id);
            }
        }
    }

    fn get_cached_static_output(&self, proc_id: SoundProcessorId) -> Option<&SoundChunk> {
        self.static_processor_cache
            .iter()
            .find(|(pid, _)| *pid == proc_id)
            .unwrap()
            .1
            .as_ref()
    }

    fn get_scratch_space(&self, size: usize) -> ScratchSlice {
        self.scratch_space.borrow_slice(size)
    }
}

impl<'a> Clone for Context<'a> {
    fn clone(&self) -> Context<'a> {
        Context::new(
            self.sound_processor_data,
            self.sound_input_data,
            self.number_source_data,
            self.number_input_data,
            self.static_processor_cache,
            self.stack.clone(),
            self.scratch_space,
        )
    }
}

pub struct ProcessorContext<'a, T: SoundState> {
    state: &'a RwLock<T>,
    state_index: usize,
    context: Context<'a>,
}

impl<'a, T: SoundState> ProcessorContext<'a, T> {
    pub(super) fn new(
        state: &'a RwLock<T>,
        state_index: usize,
        context: Context<'a>,
    ) -> ProcessorContext<'a, T> {
        ProcessorContext {
            state,
            state_index,
            context,
        }
    }

    pub fn step_single_input(&mut self, handle: &SingleSoundInputHandle, dst: &mut SoundChunk) {
        self.context.step_sound_input(handle.id(), None, dst)
    }

    pub fn step_keyed_input<K: Key, TT: SoundState>(
        &mut self,
        handle: &KeyedSoundInputHandle<K, TT>,
        key_index: usize,
        dst: &mut SoundChunk,
    ) {
        self.context
            .step_sound_input(handle.id(), Some(key_index), dst)
    }

    // pub fn single_input_state(
    //     &self,
    //     handle: &'a SingleSoundInputHandle,
    // ) -> StateTableLock<'a, EmptyState> {
    //     self.context.single_input_state(handle)
    // }

    pub fn reset_single_input(&self, handle: &SingleSoundInputHandle) {
        self.context.reset_single_input(handle);
    }

    pub fn keyed_input_state<K: Key, TT: SoundState>(
        &self,
        handle: &'a KeyedSoundInputHandle<K, TT>,
        key_index: usize,
    ) -> StateTableLock<'a, TT> {
        self.context
            .keyed_input_state(handle, self.state_index, key_index)
    }

    pub fn reset_keyed_input<K: Key, TT: SoundState>(
        &self,
        handle: &KeyedSoundInputHandle<K, TT>,
        key_index: usize,
    ) {
        self.context.reset_keyed_input(handle, key_index);
    }

    pub fn read_state(&'a self) -> RwLockReadGuard<'a, T> {
        self.state.read()
    }

    pub fn write_state<'b>(&'a self) -> RwLockWriteGuard<'a, T> {
        self.state.write()
    }

    pub fn number_context(&'a self) -> NumberContext<'a> {
        NumberContext::new(&self.context)
    }
}

#[derive(Copy, Clone)]
pub struct NumberContext<'a> {
    context: &'a Context<'a>,
}

impl<'a> NumberContext<'a> {
    pub(super) fn new(context: &'a Context<'a>) -> NumberContext<'a> {
        NumberContext { context }
    }

    // pub fn single_input_state(
    //     &self,
    //     input: &'a SingleSoundInputHandle,
    // ) -> StateTableLock<'a, EmptyState> {
    //     self.context.single_input_state(input)
    // }

    pub fn keyed_input_state<K: Key, T: SoundState>(
        &self,
        input: &'a KeyedSoundInputHandle<K, T>,
    ) -> StateTableLock<'a, T> {
        self.context.keyed_input_state_from_context(input)
    }

    pub fn sound_processor_state<T: SoundState>(
        &self,
        handle: &'a SoundProcessorData<T>,
    ) -> StateTableLock<'a, T> {
        self.context.sound_processor_state_from_context(handle)
    }

    pub fn get_scratch_space(&self, size: usize) -> ScratchSlice {
        self.context.get_scratch_space(size)
    }

    pub(super) fn evaluate_input(&self, id: NumberInputId, dst: &mut [f32]) {
        self.context.evaluate_number_input(id, dst);
    }
}
