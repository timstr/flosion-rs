use crate::core::{
    anydata::AnyData,
    engine::scratcharena::{BorrowedSlice, ScratchArena},
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::CHUNK_SIZE,
};

use super::{
    expressionargument::SoundExpressionArgumentId,
    soundinput::{InputTiming, SoundInputId},
    soundprocessor::{ProcessorState, SoundProcessorId},
};

#[derive(Clone, Copy)]
pub(crate) struct LocalArray<'a> {
    array: &'a [f32],
    argument_id: SoundExpressionArgumentId,
}

impl<'a> LocalArray<'a> {
    pub(crate) fn array(&self) -> &'a [f32] {
        self.array
    }

    pub(crate) fn argument_id(&self) -> SoundExpressionArgumentId {
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
        argument_id: SoundExpressionArgumentId,
    ) -> LocalArrayList<'a> {
        LocalArrayList {
            value: LocalArrayListValue::Containing(LocalArray { array, argument_id }, self),
        }
    }

    pub fn get(&self, argument_id: SoundExpressionArgumentId) -> &'a [f32] {
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

pub(crate) struct ProcessorStackFrame<'a> {
    parent: &'a StackFrame<'a>,
    processor_id: SoundProcessorId,
    state: &'a dyn ProcessorState,
    local_arrays: LocalArrayList<'a>,
}

impl<'a> ProcessorStackFrame<'a> {
    pub(crate) fn local_arrays(&self) -> &LocalArrayList<'a> {
        &self.local_arrays
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

pub(crate) enum StackFrame<'a> {
    Processor(ProcessorStackFrame<'a>),
    Input(InputStackFrame<'a>),
    Root,
}

impl<'a> StackFrame<'a> {
    fn find_processor_frame(&self, processor_id: SoundProcessorId) -> &ProcessorStackFrame<'a> {
        match self {
            StackFrame::Processor(p) => {
                if p.processor_id == processor_id {
                    p
                } else {
                    p.parent.find_processor_frame(processor_id)
                }
            }
            StackFrame::Input(i) => i.parent.find_processor_frame(processor_id),
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

    fn find_processor_sample_offset_and_time_speed(
        &self,
        processor_id: SoundProcessorId,
    ) -> (usize, f32) {
        match self {
            StackFrame::Processor(p) => {
                if p.processor_id == processor_id {
                    (p.state.timing().elapsed_chunks() * CHUNK_SIZE, 1.0)
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
                (o + p.state.timing().elapsed_chunks() * CHUNK_SIZE, s)
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

    pub(crate) fn stack(&self) -> &StackFrame<'a> {
        &self.stack
    }

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

    pub fn push_processor_state<T: ProcessorState>(
        &'a self,
        state: &'a T,
        local_arrays: LocalArrayList<'a>,
    ) -> Context<'a> {
        Context {
            target_processor_id: None,
            stack: StackFrame::Processor(ProcessorStackFrame {
                parent: &self.stack,
                processor_id: self.target_processor_id.unwrap(),
                state,
                local_arrays,
            }),
            scratch_space: self.scratch_space,
        }
    }

    pub(crate) fn find_processor_local_array(
        &self,
        processor_id: SoundProcessorId,
        argument_id: SoundExpressionArgumentId,
    ) -> &'a [f32] {
        self.stack
            .find_processor_frame(processor_id)
            .local_arrays
            .get(argument_id)
    }

    pub(crate) fn find_processor_state(&self, processor_id: SoundProcessorId) -> AnyData<'a> {
        let state = self.stack.find_processor_frame(processor_id).state.state();
        // TODO: remove AnyData
        AnyData::new(state)
    }

    pub fn get_scratch_space(&self, size: usize) -> BorrowedSlice {
        self.scratch_space.borrow_slice(size)
    }

    pub(crate) fn find_input_frame(&self, input_id: SoundInputId) -> &InputStackFrame {
        self.stack.find_input_frame(input_id)
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
