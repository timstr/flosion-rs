use crate::core::{
    engine::scratcharena::{BorrowedSlice, ScratchArena},
    jit::argumentstack::ArgumentStackView,
};

use super::{
    soundinput::{InputTiming, ProcessorInputId, SoundInputLocation},
    soundprocessor::{ProcessorTiming, SoundProcessorId},
};

/// Things that a sound input pushes onto the call
/// stack when it is invoked
pub(crate) struct InputFrameData<'a> {
    /// The input's id
    input_id: ProcessorInputId,

    /// The input's timing
    timing: &'a mut InputTiming,
}

impl<'a> InputFrameData<'a> {
    pub(crate) fn new(
        input_id: ProcessorInputId,
        timing: &'a mut InputTiming,
    ) -> InputFrameData<'a> {
        InputFrameData { input_id, timing }
    }

    pub(crate) fn input_id(&self) -> ProcessorInputId {
        self.input_id
    }
}

pub(crate) struct StackFrame<'a> {
    /// The parent stack frame
    parent: &'a Stack<'a>,

    /// The previous processor's id
    processor_id: SoundProcessorId,

    /// The previous processor's timing
    processor_timing: &'a ProcessorTiming,

    /// Data pushed by the invoking sound input
    input_data: InputFrameData<'a>,
}

impl<'a> StackFrame<'a> {
    pub(crate) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub(crate) fn input_data(&self) -> &InputFrameData<'a> {
        &self.input_data
    }
}

pub(crate) enum Stack<'a> {
    Frame(StackFrame<'a>),
    Root,
}

impl<'a> Stack<'a> {
    fn find_frame(&self, processor_id: SoundProcessorId) -> &StackFrame<'a> {
        match self {
            Stack::Frame(frame) => {
                if frame.processor_id == processor_id {
                    frame
                } else {
                    frame.parent.find_frame(processor_id)
                }
            }
            Stack::Root => {
                panic!("Attempted to find a processor frame which is not in the context call stack")
            }
        }
    }

    pub(crate) fn top_frame(&self) -> Option<&StackFrame<'a>> {
        match self {
            Stack::Frame(stack_frame) => Some(stack_frame),
            Stack::Root => None,
        }
    }

    pub(crate) fn top_frame_mut(&mut self) -> Option<&mut StackFrame<'a>> {
        match self {
            Stack::Frame(stack_frame) => Some(stack_frame),
            Stack::Root => None,
        }
    }
}

pub struct Context<'a> {
    current_processor_id: SoundProcessorId,
    current_processor_timing: &'a ProcessorTiming,
    scratch_arena: &'a ScratchArena,
    arguments: ArgumentStackView<'a>,
    stack: Stack<'a>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        current_processor_id: SoundProcessorId,
        current_processor_timing: &'a ProcessorTiming,
        scratch_arena: &'a ScratchArena,
        arguments: ArgumentStackView<'a>,
        stack: Stack<'a>,
    ) -> Context<'a> {
        Context {
            current_processor_id,
            current_processor_timing,
            scratch_arena,
            arguments,
            stack,
        }
    }

    pub(crate) fn current_processor_id(&self) -> SoundProcessorId {
        self.current_processor_id
    }

    pub(crate) fn current_processor_timing(&self) -> &ProcessorTiming {
        self.current_processor_timing
    }

    pub(crate) fn stack(&self) -> &Stack<'a> {
        &self.stack
    }

    pub(crate) fn push_frame(&'a self, input_data: InputFrameData<'a>) -> Stack<'a> {
        Stack::Frame(StackFrame {
            parent: &self.stack,
            processor_id: self.current_processor_id,
            processor_timing: self.current_processor_timing,
            input_data,
        })
    }

    pub(crate) fn scratch_arena(&self) -> &'a ScratchArena {
        self.scratch_arena
    }

    pub(crate) fn find_frame(&self, processor_id: SoundProcessorId) -> &StackFrame<'a> {
        self.stack.find_frame(processor_id)
    }

    pub fn get_scratch_space(&self, size: usize) -> BorrowedSlice {
        self.scratch_arena.borrow_slice(size)
    }

    pub(crate) fn argument_stack(&self) -> ArgumentStackView<'_> {
        self.arguments
    }

    pub(crate) fn time_offset_and_speed_at_processor(
        &self,
        processor_id: SoundProcessorId,
    ) -> (f32, f32) {
        todo!()
    }

    pub(crate) fn time_offset_and_speed_at_input(
        &self,
        location: SoundInputLocation,
    ) -> (f32, f32) {
        todo!()
    }

    pub fn pending_release(&self) -> Option<usize> {
        self.stack
            .top_frame()
            .and_then(|f| f.input_data.timing.pending_release())
    }

    pub fn take_pending_release(&mut self) -> Option<usize> {
        self.stack
            .top_frame_mut()
            .and_then(|f| f.input_data.timing.take_pending_release())
    }
}
