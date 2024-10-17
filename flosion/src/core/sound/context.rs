use std::any::Any;

use crate::core::engine::scratcharena::{BorrowedSlice, ScratchArena};

use super::{
    expressionargument::{CompiledProcessorArgument, ProcessorArgumentId},
    soundinput::{InputTiming, ProcessorInputId, SoundInputLocation},
    soundprocessor::{ProcessorTiming, SoundProcessorId},
};

#[derive(Clone, Copy)]
pub(crate) struct LocalArray<'a> {
    array: &'a [f32],
    argument_id: ProcessorArgumentId,
}

impl<'a> LocalArray<'a> {
    pub(crate) fn array(&self) -> &'a [f32] {
        self.array
    }

    pub(crate) fn argument_id(&self) -> ProcessorArgumentId {
        self.argument_id
    }
}

#[derive(Clone, Copy)]
enum LocalArrayListValue<'a> {
    Empty,
    Containing(LocalArray<'a>, &'a LocalArrayList<'a>),
}

#[derive(Clone, Copy)]
pub struct LocalArrayList<'a> {
    value: LocalArrayListValue<'a>,
}

impl<'a> LocalArrayList<'a> {
    pub fn new() -> LocalArrayList<'a> {
        LocalArrayList {
            value: LocalArrayListValue::Empty,
        }
    }

    pub fn push(
        &'a self,
        array: &'a [f32],
        argument: &CompiledProcessorArgument,
    ) -> LocalArrayList<'a> {
        LocalArrayList {
            value: LocalArrayListValue::Containing(
                LocalArray {
                    array,
                    argument_id: argument.id(),
                },
                self,
            ),
        }
    }

    pub fn get(&self, argument_id: ProcessorArgumentId) -> &'a [f32] {
        match &self.value {
            LocalArrayListValue::Empty => {
                panic!("Attempted to get a LocalArray which was never pushed")
            }
            LocalArrayListValue::Containing(local_array, rest_of_list) => {
                if local_array.argument_id == argument_id {
                    local_array.array
                } else {
                    rest_of_list.get(argument_id)
                }
            }
        }
    }

    pub(crate) fn as_vec(&self) -> Vec<LocalArray<'a>> {
        fn visit<'b>(list: &LocalArrayList<'b>, vec: &mut Vec<LocalArray<'b>>) {
            match list.value {
                LocalArrayListValue::Empty => (),
                LocalArrayListValue::Containing(v, rest) => {
                    vec.push(v.clone());
                    visit(rest, vec);
                }
            }
        }

        let mut vec = Vec::new();
        visit(self, &mut vec);
        vec
    }
}

/// Things that a sound processor pushes onto the call
/// stack when it invokes one of its sound inputs
#[derive(Copy, Clone)]
pub(crate) struct ProcessorFrameData<'a> {
    /// The processor's state
    state: Option<&'a dyn Any>,

    /// Any local arrays that the processor pushed
    local_arrays: LocalArrayList<'a>,
}

impl<'a> ProcessorFrameData<'a> {
    pub(crate) fn new(
        state: Option<&'a dyn Any>,
        local_arrays: LocalArrayList<'a>,
    ) -> ProcessorFrameData<'a> {
        ProcessorFrameData {
            state,
            local_arrays,
        }
    }

    pub(crate) fn state(&self) -> Option<&'a dyn Any> {
        self.state
    }

    pub(crate) fn local_arrays(&self) -> LocalArrayList<'a> {
        self.local_arrays
    }
}

/// Things that a sound input pushes onto the call
/// stack when it is invoked
pub(crate) struct InputFrameData<'a> {
    /// The input's id
    input_id: ProcessorInputId,

    /// The input's state
    state: &'a dyn Any,

    /// The input's timing
    timing: &'a mut InputTiming,
}

impl<'a> InputFrameData<'a> {
    pub(crate) fn new(
        input_id: ProcessorInputId,
        state: &'a dyn Any,
        timing: &'a mut InputTiming,
    ) -> InputFrameData<'a> {
        InputFrameData {
            input_id,
            state,
            timing,
        }
    }

    pub(crate) fn input_id(&self) -> ProcessorInputId {
        self.input_id
    }

    pub(crate) fn state(&self) -> &'a dyn Any {
        self.state
    }
}

pub(crate) struct StackFrame<'a> {
    /// The parent stack frame
    parent: &'a Stack<'a>,

    /// The previous processor's id
    processor_id: SoundProcessorId,

    /// The previous processor's timing
    processor_timing: &'a ProcessorTiming,

    /// Data pushed by the invoking sound processor
    processor_data: ProcessorFrameData<'a>,

    /// Data pushed by the invoking sound input
    input_data: InputFrameData<'a>,
}

impl<'a> StackFrame<'a> {
    pub(crate) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub(crate) fn processor_data(&self) -> &ProcessorFrameData<'a> {
        &self.processor_data
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
    stack: Stack<'a>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        current_processor_id: SoundProcessorId,
        current_processor_timing: &'a ProcessorTiming,
        scratch_arena: &'a ScratchArena,
        stack: Stack<'a>,
    ) -> Context<'a> {
        Context {
            current_processor_id,
            current_processor_timing,
            scratch_arena,
            stack,
        }
    }

    pub(crate) fn current_processor_id(&self) -> SoundProcessorId {
        self.current_processor_id
    }

    pub(crate) fn stack(&self) -> &Stack<'a> {
        &self.stack
    }

    pub(crate) fn push_frame(
        &'a self,
        processor_data: ProcessorFrameData<'a>,
        input_data: InputFrameData<'a>,
    ) -> Stack<'a> {
        Stack::Frame(StackFrame {
            parent: &self.stack,
            processor_id: self.current_processor_id,
            processor_timing: self.current_processor_timing,
            processor_data,
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
