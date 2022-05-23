use std::collections::HashMap;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::core::{soundchunk::CHUNK_SIZE, soundinput::SoundInputWrapper};

use super::{
    key::Key,
    numberinput::NumberInputId,
    numbersource::{NumberConfig, NumberSourceId},
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    scratcharena::{ScratchArena, ScratchSlice},
    soundchunk::SoundChunk,
    soundengine::{
        EngineNumberInputData, EngineNumberSourceData, EngineSoundInputData,
        EngineSoundProcessorData,
    },
    soundinput::{KeyedSoundInputHandle, SingleSoundInputHandle, SoundInputId, SoundInputState},
    soundprocessor::{SoundProcessorData, SoundProcessorId},
    soundstate::{EmptyState, SoundState},
    statetable::TableLock,
};

#[derive(Copy, Clone)]
pub struct SoundProcessorFrame {
    pub id: SoundProcessorId,
    pub state_index: usize,
}

#[derive(Copy, Clone)]
pub struct SoundInputFrame {
    pub id: SoundInputId,
    pub key_index: usize,
    pub state_index: usize,
}

#[derive(Copy, Clone)]
pub enum SoundStackFrame {
    Processor(SoundProcessorFrame),
    Input(SoundInputFrame),
}

impl SoundStackFrame {
    pub fn into_processor_frame(self) -> Option<SoundProcessorFrame> {
        match self {
            SoundStackFrame::Processor(f) => Some(f),
            _ => None,
        }
    }

    pub fn into_input_frame(self) -> Option<SoundInputFrame> {
        match self {
            SoundStackFrame::Input(f) => Some(f),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq)]
enum SoundId {
    Processor(SoundProcessorId),
    Input(SoundInputId),
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

    fn reset_sound_processor(
        &self,
        processor_id: SoundProcessorId,
        dst_input: SoundInputId,
        dst_state_index: usize,
    ) {
        let data = self.sound_processor_data.get(&processor_id).unwrap();
        data.wrapper().reset_state(dst_input, dst_state_index);
        let i = data.wrapper().find_state_index(dst_input, dst_state_index);
        for proc_input_id in data.inputs() {
            let input_data = self.sound_input_data.get(&proc_input_id).unwrap();
            input_data.input().require_reset_states(i);
        }
    }

    fn current_processor_frame(&self) -> SoundProcessorFrame {
        self.current_frame().into_processor_frame().unwrap()
    }

    pub(super) fn single_input_state_from_context(
        &self,
        input: &'a SingleSoundInputHandle,
    ) -> TableLock<'a, SoundInputState<EmptyState>> {
        let input_id = input.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::Input(i) => i.id == input_id,
                _ => false,
            })
            .expect(
                "Failed to find a SingleSoundInput's call frame in the the context's call stack",
            );
        let f = f.into_input_frame().expect("Found a call frame in the context with the correct input id which is somehow the wrong type");
        input.input().get_state(f.state_index)
    }

    pub(super) fn reset_input(
        &self,
        input_id: SoundInputId,
        key_index: usize,
        delay_from_chunk_start: usize,
    ) {
        debug_assert!(delay_from_chunk_start < CHUNK_SIZE);
        let input_data = self.sound_input_data.get(&input_id).unwrap();
        let the_input = input_data.input();
        let state_index = self.current_processor_frame().state_index;
        the_input.reset_state(state_index, key_index, delay_from_chunk_start);
        if let Some(spid) = input_data.target() {
            self.reset_sound_processor(spid, input_id, state_index);
        }
    }

    pub(super) fn keyed_input_state_from_context<K: Key, T: SoundState>(
        &self,
        input: &'a KeyedSoundInputHandle<K, T>,
    ) -> TableLock<'a, SoundInputState<T>> {
        let input_id = input.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::Input(i) => i.id == input_id,
                _ => false,
            })
            .expect("Failed to find a KeyedSoundInput's call frame in the context's call stack");
        let f = f.into_input_frame().expect("Found a call frame in the context with the correct input id which is somehow the wrong type");
        input.input().get_state(f.state_index, f.key_index)
    }

    pub(super) fn sound_processor_state_from_context<T: SoundState>(
        &self,
        handle: &'a SoundProcessorData<T>,
    ) -> TableLock<'a, T> {
        let proc_id = handle.id();
        let f = self
            .stack
            .iter()
            .rev()
            .find(|f| match f {
                SoundStackFrame::Processor(i) => i.id == proc_id,
                _ => false,
            })
            .expect("Failed to find a SoundProcessor's call frame in the context's call stack");
        let f = f.into_processor_frame().expect("Found a call frame in the context with the correct processor id which is somehow the wrong type");
        handle.get_state(f.state_index)
    }

    fn current_time_at(&self, sound_id: SoundId, dst: &mut [f32], config: NumberConfig) {
        let mut num_chunks: usize = 0;
        let mut num_samples: usize = config.sample_offset();
        let mut found = false;
        for f in &self.stack {
            match f {
                SoundStackFrame::Input(input_frame) => {
                    if SoundId::Input(input_frame.id) == sound_id {
                        found = true;
                        break;
                    }
                    let input_data = self.sound_input_data.get(&input_frame.id).unwrap();
                    let input_time = input_data
                        .input()
                        .get_state_time(input_frame.state_index, input_frame.key_index);
                    num_chunks += input_time.elapsed_chunks();
                    num_samples += input_time.sample_offset();
                }
                SoundStackFrame::Processor(proc_frame) => {
                    if SoundId::Processor(proc_frame.id) == sound_id {
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            panic!("Attempted to find the current time at a sound input or processor which was not found in the call stack");
        }
        let total_samples = num_chunks * CHUNK_SIZE + num_samples;
        let sample_duration = 1.0 / SAMPLE_FREQUENCY as f32;
        let start_time = total_samples as f32 * sample_duration;
        if config.is_samplewise_temporal() {
            let end_time = (total_samples + dst.len()) as f32 * sample_duration;
            numeric::linspace(dst, start_time, end_time);
        } else {
            numeric::fill(dst, start_time);
        }
    }

    pub(super) fn evaluate_number_input(
        &self,
        input_id: NumberInputId,
        dst: &mut [f32],
        config: NumberConfig,
    ) {
        let input_data = self.number_input_data.get(&input_id).expect(
            "Failed to find the data for a number input while evaluating it in an audio context",
        );
        match input_data.target() {
            Some(nsid) => {
                let source_data = self.number_source_data.get(&nsid).expect("Failed to find the data for a number source pointed to by a number input being evaluated in an audio context");
                source_data
                    .instance()
                    .eval(dst, NumberContext::new(self, config));
            }
            None => numeric::fill(dst, 0.0),
        }
    }

    pub fn current_frame(&self) -> SoundStackFrame {
        *self
            .stack
            .last()
            .expect("Attempted to get the current frame from an empty audio context call stack")
    }

    fn step_sound_input(
        &mut self,
        input_id: SoundInputId,
        key_index: Option<usize>,
        dst: &mut SoundChunk,
    ) {
        let frame = self.current_processor_frame();
        let input = self.sound_input_data.get(&input_id).unwrap();
        let effective_key_index = key_index.unwrap_or(0);
        if input
            .input()
            .state_needs_reset(frame.state_index, effective_key_index)
        {
            panic!("Attempted to step a sound input which needs to be reset first. Please make sure to reset the sound input before using it.");
        }
        if let Some(target) = input.target() {
            let other_pd = self.sound_processor_data.get(&target).expect("Failed to find data for a sound processor pointed to by a sound input in an audio context");
            let other_proc = other_pd.wrapper();
            if other_proc.is_static() {
                let ch = self.get_cached_static_output(other_pd.id()).expect("Failed to find a cached static processor output pointed to by a sound input an an audio context");
                dst.copy_from(ch);
            } else {
                // TODO: avoid this clone
                let mut other_stack = self.stack.clone();

                other_stack.push(SoundStackFrame::Input(SoundInputFrame {
                    id: input_id,
                    state_index: frame.state_index,
                    key_index: effective_key_index,
                }));

                let input_state_index = input
                    .input()
                    .get_state_index(frame.state_index, effective_key_index);
                let other_proc_state_index =
                    other_proc.find_state_index(input_id, input_state_index);
                other_stack.push(SoundStackFrame::Processor(SoundProcessorFrame {
                    id: other_proc.id(),
                    state_index: other_proc_state_index,
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
                input
                    .input()
                    .advance_timing_one_chunk(frame.state_index, effective_key_index);
            }
        } else {
            dst.silence();
        }
    }

    fn get_cached_static_output(&self, proc_id: SoundProcessorId) -> Option<&SoundChunk> {
        self.static_processor_cache
            .iter()
            .find(|(pid, _)| *pid == proc_id)
            .expect("Failed to find a cached static processor output in an audio context")
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

    pub fn single_input_needs_reset(&self, handle: &SingleSoundInputHandle) -> bool {
        handle.input().state_needs_reset(self.state_index, 0)
    }

    pub fn keyed_input_needs_reset<K: Key, TT: SoundState>(
        &self,
        handle: &KeyedSoundInputHandle<K, TT>,
        key_index: usize,
    ) -> bool {
        handle
            .input()
            .state_needs_reset(self.state_index, key_index)
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

    pub fn single_input_state(
        &self,
        handle: &'a SingleSoundInputHandle,
    ) -> TableLock<'a, SoundInputState<EmptyState>> {
        handle.input().get_state(self.state_index)
    }

    pub fn keyed_input_state<K: Key, TT: SoundState>(
        &self,
        handle: &'a KeyedSoundInputHandle<K, TT>,
        key_index: usize,
    ) -> TableLock<'a, SoundInputState<TT>> {
        handle.input().get_state(self.state_index, key_index)
    }

    pub fn reset_single_input(
        &self,
        handle: &SingleSoundInputHandle,
        delay_from_chunk_start: usize,
    ) {
        self.context
            .reset_input(handle.id(), 0, delay_from_chunk_start)
    }

    pub fn reset_keyed_input<K: Key, TT: SoundState>(
        &self,
        handle: &KeyedSoundInputHandle<K, TT>,
        key_index: usize,
        delay_from_chunk_start: usize,
    ) {
        self.context
            .reset_input(handle.id(), key_index, delay_from_chunk_start)
    }

    pub fn read_state(&'a self) -> RwLockReadGuard<'a, T> {
        self.state.read()
    }

    pub fn write_state<'b>(&'a self) -> RwLockWriteGuard<'a, T> {
        self.state.write()
    }

    pub fn number_context(&'a self, config: NumberConfig) -> NumberContext<'a> {
        NumberContext::new(&self.context, config)
    }
}

#[derive(Copy, Clone)]
pub struct NumberContext<'a> {
    context: &'a Context<'a>,
    config: NumberConfig,
}

impl<'a> NumberContext<'a> {
    pub(super) fn new(context: &'a Context<'a>, config: NumberConfig) -> NumberContext<'a> {
        NumberContext { context, config }
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
    ) -> TableLock<'a, SoundInputState<T>> {
        self.context.keyed_input_state_from_context(input)
    }

    pub fn sound_processor_state<T: SoundState>(
        &self,
        handle: &'a SoundProcessorData<T>,
    ) -> TableLock<'a, T> {
        self.context.sound_processor_state_from_context(handle)
    }

    pub fn current_time_at_sound_input(&self, input_id: SoundInputId, dst: &mut [f32]) {
        self.context
            .current_time_at(SoundId::Input(input_id), dst, self.config);
    }

    pub fn current_time_at_sound_processor(&self, proc_id: SoundProcessorId, dst: &mut [f32]) {
        self.context
            .current_time_at(SoundId::Processor(proc_id), dst, self.config);
    }

    pub fn get_scratch_space(&self, size: usize) -> ScratchSlice {
        self.context.get_scratch_space(size)
    }

    pub(super) fn evaluate_input(&self, id: NumberInputId, dst: &mut [f32]) {
        self.context.evaluate_number_input(id, dst, self.config);
    }
}
