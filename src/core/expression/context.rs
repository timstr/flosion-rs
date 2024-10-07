use std::any::Any;

use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    sound::{
        context::{Context, LocalArrayList},
        expressionargument::ProcessorArgumentId,
        soundinput::SoundInputLocation,
        soundprocessor::SoundProcessorId,
    },
};

pub struct ExpressionContext<'a> {
    audio_context: Context<'a>,
    top_processor_state: Option<&'a dyn Any>,
    top_processor_arrays: LocalArrayList<'a>,
}

impl<'a> ExpressionContext<'a> {
    pub fn new_minimal(audio_context: Context<'a>) -> ExpressionContext<'a> {
        ExpressionContext {
            audio_context,
            top_processor_state: None,
            top_processor_arrays: LocalArrayList::new(),
        }
    }

    pub fn new_with_state(
        audio_context: Context<'a>,
        processor_state: &'a dyn Any,
    ) -> ExpressionContext<'a> {
        ExpressionContext {
            audio_context,
            top_processor_state: Some(processor_state),
            top_processor_arrays: LocalArrayList::new(),
        }
    }

    pub fn new_with_arrays(
        audio_context: Context<'a>,
        arrays: LocalArrayList<'a>,
    ) -> ExpressionContext<'a> {
        ExpressionContext {
            audio_context,
            top_processor_state: None,
            top_processor_arrays: arrays,
        }
    }

    pub fn new_with_state_and_arrays(
        audio_context: Context<'a>,
        processor_state: &'a dyn Any,
        arrays: LocalArrayList<'a>,
    ) -> ExpressionContext<'a> {
        ExpressionContext {
            audio_context,
            top_processor_state: Some(processor_state),
            top_processor_arrays: arrays,
        }
    }

    pub(crate) fn top_processor_state(&self) -> Option<&'a dyn Any> {
        self.top_processor_state
    }

    pub(crate) fn top_processor_arrays(&self) -> &LocalArrayList {
        &self.top_processor_arrays
    }

    pub(crate) fn read_scalar_from_input_state(
        &self,
        location: SoundInputLocation,
        read_fn: ScalarReadFunc,
    ) -> f32 {
        let frame = self.audio_context.find_frame(location.processor());
        debug_assert_eq!(frame.input_data().input_id(), location.input());
        read_fn(frame.input_data().state())
    }

    pub(crate) fn read_scalar_from_processor_state(
        &self,
        id: SoundProcessorId,
        read_fn: ScalarReadFunc,
    ) -> f32 {
        let state = if self.audio_context.current_processor_id() == id {
            self.top_processor_state.expect(
                "Attempted to read state from current processor \
                which was not provided to ExpressionContext",
            )
        } else {
            let frame = self.audio_context.find_frame(id);
            frame.processor_data().state().expect(
                "Attempted to read state from processor which did \
                not provide any while invoking sound input",
            )
        };
        read_fn(state)
    }

    pub(crate) fn read_array_from_input_state(
        &self,
        location: SoundInputLocation,
        read_fn: ArrayReadFunc,
    ) -> &[f32] {
        let frame = self.audio_context.find_frame(location.processor());
        debug_assert_eq!(frame.input_data().input_id(), location.input());
        read_fn(frame.input_data().state())
    }

    pub(crate) fn read_array_from_processor_state(
        &self,
        id: SoundProcessorId,
        read_fn: ArrayReadFunc,
    ) -> &[f32] {
        let state = if self.audio_context.current_processor_id() == id {
            self.top_processor_state.expect(
                "Attempted to read state from current processor \
                which was not provided to ExpressionContext",
            )
        } else {
            let frame = self.audio_context.find_frame(id);
            frame.processor_data().state().expect(
                "Attempted to read state from processor which did \
                not provide any while invoking sound input",
            )
        };
        read_fn(state)
    }

    pub(crate) fn read_local_array_from_sound_processor(
        &self,
        proc_id: SoundProcessorId,
        arg_id: ProcessorArgumentId,
    ) -> &[f32] {
        let frame = self.audio_context.find_frame(proc_id);
        frame.processor_data().local_arrays().get(arg_id)
    }

    pub(crate) fn get_time_and_speed_at_sound_input(
        &self,
        location: SoundInputLocation,
    ) -> (f32, f32) {
        self.audio_context.time_offset_and_speed_at_input(location)
    }

    pub(crate) fn get_time_and_speed_at_sound_processor(&self, id: SoundProcessorId) -> (f32, f32) {
        self.audio_context.time_offset_and_speed_at_processor(id)
    }
}
