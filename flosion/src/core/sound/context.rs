use crate::core::{
    engine::scratcharena::{BorrowedSlice, ScratchArena},
    jit::argumentstack::ArgumentStackView,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::CHUNK_SIZE,
};

use super::{
    soundinput::{InputTiming, ProcessorInputId, SoundInputLocation},
    soundprocessor::{ProcessorTiming, SoundProcessorId},
};

pub(crate) struct AudioStackFrame<'a> {
    /// The parent stack frame
    parent: &'a AudioStack<'a>,

    /// The previous processor's id
    processor_id: SoundProcessorId,

    /// The previous processor's timing
    processor_timing: &'a ProcessorTiming,

    /// The input's id
    input_id: ProcessorInputId,

    /// The input's timing
    input_timing: &'a mut InputTiming,
}

impl<'a> AudioStackFrame<'a> {
    pub(crate) fn input_location(&self) -> SoundInputLocation {
        SoundInputLocation::new(self.processor_id, self.input_id)
    }
}

// TODO: rename
pub(crate) enum AudioStack<'a> {
    Frame(AudioStackFrame<'a>),
    Root,
}

impl<'a> AudioStack<'a> {
    pub(crate) fn top_frame(&self) -> Option<&AudioStackFrame<'a>> {
        match self {
            AudioStack::Frame(stack_frame) => Some(stack_frame),
            AudioStack::Root => None,
        }
    }

    pub(crate) fn top_frame_mut(&mut self) -> Option<&mut AudioStackFrame<'a>> {
        match self {
            AudioStack::Frame(stack_frame) => Some(stack_frame),
            AudioStack::Root => None,
        }
    }

    fn elapsed_samples_and_speed_from_processor(
        &self,
        processor_id: SoundProcessorId,
    ) -> (usize, f32) {
        match self {
            AudioStack::Frame(stack_frame) => {
                if stack_frame.processor_id == processor_id {
                    let elapsed_samples = stack_frame.processor_timing.elapsed_chunks()
                        * CHUNK_SIZE
                        + stack_frame.input_timing.sample_offset();
                    let speed = stack_frame.input_timing.time_speed();
                    (elapsed_samples, speed)
                } else {
                    let (other_elapsed_samples, other_speed) = stack_frame
                        .parent
                        .elapsed_samples_and_speed_from_processor(processor_id);
                    (
                        other_elapsed_samples + stack_frame.input_timing.sample_offset(),
                        other_speed * stack_frame.input_timing.time_speed(),
                    )
                }
            }
            AudioStack::Root => {
                panic!("Attempted to get timing of a processor which is not on the stack")
            }
        }
    }

    fn elapsed_samples_and_speed_from_input(
        &self,
        input_location: SoundInputLocation,
        child_samples: usize,
    ) -> (usize, f32) {
        match self {
            AudioStack::Frame(stack_frame) => {
                if input_location == stack_frame.input_location() {
                    (child_samples, 1.0)
                } else {
                    let (other_elapsed_samples, other_speed) =
                        stack_frame.parent.elapsed_samples_and_speed_from_input(
                            input_location,
                            stack_frame.processor_timing.elapsed_chunks() * CHUNK_SIZE,
                        );
                    (
                        other_elapsed_samples + stack_frame.input_timing.sample_offset(),
                        other_speed * stack_frame.input_timing.time_speed(),
                    )
                }
            }
            AudioStack::Root => {
                panic!("Attempted to get timing of a processor which is not on the stack")
            }
        }
    }
}

pub struct Context<'a> {
    current_processor_id: SoundProcessorId,
    current_processor_timing: &'a ProcessorTiming,
    scratch_arena: &'a ScratchArena,
    arguments: ArgumentStackView<'a>,
    stack: AudioStack<'a>,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        current_processor_id: SoundProcessorId,
        current_processor_timing: &'a ProcessorTiming,
        scratch_arena: &'a ScratchArena,
        arguments: ArgumentStackView<'a>,
        stack: AudioStack<'a>,
    ) -> Context<'a> {
        Context {
            current_processor_id,
            current_processor_timing,
            scratch_arena,
            arguments,
            stack,
        }
    }

    pub(crate) fn current_processor_timing(&self) -> &ProcessorTiming {
        self.current_processor_timing
    }

    pub(crate) fn push_frame(
        &'a self,
        input_id: ProcessorInputId,
        input_timing: &'a mut InputTiming,
    ) -> AudioStack<'a> {
        AudioStack::Frame(AudioStackFrame {
            parent: &self.stack,
            processor_id: self.current_processor_id,
            processor_timing: self.current_processor_timing,
            input_id,
            input_timing,
        })
    }

    pub(crate) fn scratch_arena(&self) -> &'a ScratchArena {
        self.scratch_arena
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
        let (elapsed_samples, speed) = if processor_id == self.current_processor_id {
            (
                self.current_processor_timing.elapsed_chunks() * CHUNK_SIZE,
                1.0,
            )
        } else {
            self.stack
                .elapsed_samples_and_speed_from_processor(processor_id)
        };

        (
            elapsed_samples as f32 / SAMPLE_FREQUENCY as f32,
            1.0 / speed,
        )
    }

    pub(crate) fn time_offset_and_speed_at_input(
        &self,
        location: SoundInputLocation,
    ) -> (f32, f32) {
        let (elapsed_samples, speed) = self.stack.elapsed_samples_and_speed_from_input(
            location,
            self.current_processor_timing.elapsed_chunks() * CHUNK_SIZE,
        );
        (
            elapsed_samples as f32 / SAMPLE_FREQUENCY as f32,
            1.0 / speed,
        )
    }

    pub fn pending_release(&self) -> Option<usize> {
        self.stack
            .top_frame()
            .and_then(|f| f.input_timing.pending_release())
    }

    pub fn take_pending_release(&mut self) -> Option<usize> {
        self.stack
            .top_frame_mut()
            .and_then(|f| f.input_timing.take_pending_release())
    }
}
