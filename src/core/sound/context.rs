use crate::core::{
    anydata::AnyData,
    engine::scratcharena::{BorrowedSlice, ScratchArena},
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::CHUNK_SIZE,
};

use super::{
    soundinput::{InputTiming, SoundInputId},
    soundprocessor::{ProcessorState, SoundProcessorId},
};

pub(crate) struct ProcessorStackFrame<'a> {
    parent: &'a StackFrame<'a>,
    processor_id: SoundProcessorId,
    data: &'a dyn ProcessorState,
}

impl<'a> ProcessorStackFrame<'a> {
    pub(super) fn data(&self) -> &'a dyn ProcessorState {
        self.data
    }
}

pub(crate) struct InputStackFrame<'a> {
    parent: &'a StackFrame<'a>,
    input_id: SoundInputId,
    state: AnyData<'a>,
    timing: &'a mut InputTiming,
}

impl<'a> InputStackFrame<'a> {
    pub(crate) fn state(&self) -> &AnyData<'a> {
        &self.state
    }

    pub(crate) fn timing(&'a self) -> &'a InputTiming {
        self.timing
    }

    pub(super) fn take_pending_release(&mut self) -> Option<usize> {
        self.timing.take_pending_release()
    }
}

enum StackFrame<'a> {
    Processor(ProcessorStackFrame<'a>),
    Input(InputStackFrame<'a>),
    Root,
}

impl<'a> StackFrame<'a> {
    fn find_processor_state(&self, processor_id: SoundProcessorId) -> AnyData<'a> {
        match self {
            StackFrame::Processor(p) => {
                if p.processor_id == processor_id {
                    AnyData::new(p.data().state())
                } else {
                    p.parent.find_processor_state(processor_id)
                }
            }
            StackFrame::Input(i) => i.parent.find_processor_state(processor_id),
            StackFrame::Root => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_frame(&self, input_id: SoundInputId) -> &InputStackFrame {
        match self {
            StackFrame::Processor(p) => p.parent.find_input_frame(input_id),
            StackFrame::Input(i) => {
                if i.input_id == input_id {
                    i
                } else {
                    i.parent.find_input_frame(input_id)
                }
            }
            StackFrame::Root => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }

    fn find_processor_sample_offset(&self, processor_id: SoundProcessorId) -> usize {
        match self {
            StackFrame::Processor(p) => {
                let s = match p.data().timing() {
                    Some(t) => t.elapsed_chunks() * CHUNK_SIZE,
                    None => panic!("Attempted to get timing for a static processor, which is not (yet) supported"),
                };
                if p.processor_id == processor_id {
                    s
                } else {
                    s + p.parent.find_processor_sample_offset(processor_id)
                }
            }
            StackFrame::Input(i) => {
                i.timing.sample_offset() + i.parent.find_processor_sample_offset(processor_id)
            }
            StackFrame::Root => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_sample_offset(&self, input_id: SoundInputId) -> usize {
        match self {
            StackFrame::Processor(p) => {
                let s = match p.data().timing() {
                    Some(t) => t.elapsed_chunks() * CHUNK_SIZE,
                    None => panic!("Attempted to get timing for a static processor, which is not (yet) supported"),
                };
                s + p.parent.find_input_sample_offset(input_id)
            }
            StackFrame::Input(i) => {
                if i.input_id == input_id {
                    0
                } else {
                    i.timing().sample_offset() + i.parent.find_input_sample_offset(input_id)
                }
            }
            StackFrame::Root => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }

    fn find_processor_sample_offset_and_time_speed(
        &self,
        processor_id: SoundProcessorId,
    ) -> (usize, f32) {
        match self {
            StackFrame::Processor(p) => {
                if p.processor_id == processor_id {
                    (p.data.timing().unwrap().elapsed_chunks() * CHUNK_SIZE, 1.0)
                } else {
                    p.parent
                        .find_processor_sample_offset_and_time_speed(processor_id)
                }
            }
            StackFrame::Input(i) => {
                let (o, s) = i
                    .parent
                    .find_processor_sample_offset_and_time_speed(processor_id);
                (o + i.timing.sample_offset(), s * i.timing.time_speed())
            }
            StackFrame::Root => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    fn find_input_sample_offset_and_time_speed(&self, input_id: SoundInputId) -> (usize, f32) {
        match self {
            StackFrame::Processor(p) => {
                let (o, s) = p.parent.find_input_sample_offset_and_time_speed(input_id);
                (
                    o + p.data.timing().unwrap().elapsed_chunks() * CHUNK_SIZE,
                    s,
                )
            }
            StackFrame::Input(i) => {
                if i.input_id == input_id {
                    (0, 1.0)
                } else {
                    let (o, s) = i.parent.find_input_sample_offset_and_time_speed(input_id);
                    (o + i.timing.sample_offset(), s * i.timing.time_speed())
                }
            }
            StackFrame::Root => {
                panic!("Attempted to find an input frame which is not in the context call stack")
            }
        }
    }
}

pub struct Context<'a> {
    target_processor_id: Option<SoundProcessorId>,
    // TODO: delete scratch_space
    scratch_space: &'a ScratchArena,
    stack: StackFrame<'a>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        target_processor_id: SoundProcessorId,
        scratch_space: &'a ScratchArena,
    ) -> Context<'a> {
        Context {
            target_processor_id: Some(target_processor_id),
            scratch_space,
            stack: StackFrame::Root,
        }
    }

    // TODO: it appears that input states and processor states are always pushed a pair at a time.
    // Consider avoiding half the indirections by storing both in the same frame, and by making
    // the root frame always correspond to a static processor without a target input
    pub(crate) fn push_input(
        &'a self,
        target: Option<SoundProcessorId>,
        input_id: SoundInputId,
        state: AnyData<'a>,
        timing: &'a mut InputTiming,
    ) -> Context<'a> {
        Context {
            target_processor_id: target,
            stack: StackFrame::Input(InputStackFrame {
                parent: &self.stack,
                input_id,
                state,
                timing,
            }),
            scratch_space: self.scratch_space,
        }
    }

    pub fn push_processor_state<T: ProcessorState>(&'a self, state: &'a T) -> Context<'a> {
        Context {
            target_processor_id: None,
            stack: StackFrame::Processor(ProcessorStackFrame {
                parent: &self.stack,
                processor_id: self.target_processor_id.unwrap(),
                data: state,
            }),
            scratch_space: self.scratch_space,
        }
    }

    pub(crate) fn find_processor_state(&self, processor_id: SoundProcessorId) -> AnyData<'a> {
        self.stack.find_processor_state(processor_id)
    }

    pub fn get_scratch_space(&self, size: usize) -> BorrowedSlice {
        self.scratch_space.borrow_slice(size)
    }

    pub(crate) fn find_input_frame(&self, input_id: SoundInputId) -> &InputStackFrame {
        self.stack.find_input_frame(input_id)
    }

    fn current_time_impl(samples_at_start: usize, dst: &mut [f32]) {
        let seconds_per_sample = 1.0 / SAMPLE_FREQUENCY as f32;
        let t0 = samples_at_start as f32 * seconds_per_sample;
        let t1 = (samples_at_start + dst.len()) as f32 * seconds_per_sample;
        // TODO: check whether dst is temporal or not
        numeric::linspace(dst, t0, t1);
    }

    pub(crate) fn current_time_at_sound_processor(
        &self,
        processor_id: SoundProcessorId,
        dst: &mut [f32],
    ) {
        let s = self.stack.find_processor_sample_offset(processor_id);
        Self::current_time_impl(s, dst);
    }

    pub(crate) fn current_time_at_sound_input(&self, input_id: SoundInputId, dst: &mut [f32]) {
        let s = self.stack.find_input_sample_offset(input_id);
        Self::current_time_impl(s, dst);
    }

    pub(crate) fn time_offset_and_speed_at_processor(
        &self,
        processor_id: SoundProcessorId,
    ) -> (f32, f32) {
        let (samples, speed) = self
            .stack
            .find_processor_sample_offset_and_time_speed(processor_id);
        (samples as f32 / SAMPLE_FREQUENCY as f32, speed)
    }

    pub(crate) fn time_offset_and_speed_at_input(&self, input_id: SoundInputId) -> (f32, f32) {
        let (samples, speed) = self.stack.find_input_sample_offset_and_time_speed(input_id);
        (samples as f32 / SAMPLE_FREQUENCY as f32, speed)
    }

    pub fn pending_release(&self) -> Option<usize> {
        if let StackFrame::Input(i) = &self.stack {
            i.timing().pending_release()
        } else {
            debug_assert!(false);
            None
        }
    }

    pub fn take_pending_release(&mut self) -> Option<usize> {
        if let StackFrame::Input(i) = &mut self.stack {
            i.take_pending_release()
        } else {
            debug_assert!(false);
            None
        }
    }
}
