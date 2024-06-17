use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    sound::{
        context::Context, expressionargument::SoundExpressionArgumentId, soundinput::SoundInputId,
        soundprocessor::SoundProcessorId,
    },
};

/// The set of outside information that is available to an expression while
/// it is being evaluated. This is intended to encompass time-varying state
/// belonging to sound processors and sound inputs up the audio call stack.
// NOTE: this is currently a trait and not a simple struct (see
// `sound::Context`) because the same compiled expressions get used both
// during audio processing and in the gui, where they don't have access
// to the full audio call stack.
pub(crate) trait ExpressionContext {
    /// Read a single value from the state of a sound input using the given function
    fn read_scalar_from_sound_input(&self, id: SoundInputId, read_fn: ScalarReadFunc) -> f32;

    /// Read a single value from the state of a sound processor using the given function
    fn read_scalar_from_sound_processor(
        &self,
        id: SoundProcessorId,
        read_fn: ScalarReadFunc,
    ) -> f32;

    /// Read an array of values from the state of a sound input using the given function
    fn read_array_from_sound_input(&self, id: SoundInputId, read_fn: ArrayReadFunc) -> &[f32];

    /// Read an array of values from the state of a sound processor using the given function
    fn read_array_from_sound_processor(
        &self,
        id: SoundProcessorId,
        read_fn: ArrayReadFunc,
    ) -> &[f32];

    /// Read an array of values from a processor's local scope
    fn read_local_array_from_sound_processor(
        &self,
        id: SoundProcessorId,
        source_id: SoundExpressionArgumentId,
    ) -> &[f32];

    /// Get the time and speed of time at the given sound input
    fn get_time_and_speed_at_sound_input(&self, id: SoundInputId) -> (f32, f32);

    /// Get the time and speed of time at the given sound processor
    fn get_time_and_speed_at_sound_processor(&self, id: SoundProcessorId) -> (f32, f32);
}

impl<'ctx> ExpressionContext for Context<'ctx> {
    fn read_scalar_from_sound_input(&self, id: SoundInputId, read_fn: ScalarReadFunc) -> f32 {
        let frame = self.find_input_frame(id);
        read_fn(&frame.state())
    }

    fn read_scalar_from_sound_processor(
        &self,
        id: SoundProcessorId,
        read_fn: ScalarReadFunc,
    ) -> f32 {
        let frame = self.find_processor_state(id);
        read_fn(&frame)
    }

    fn read_array_from_sound_input(&self, id: SoundInputId, read_fn: ArrayReadFunc) -> &[f32] {
        let frame = self.find_input_frame(id);
        read_fn(&frame.state())
    }

    fn read_array_from_sound_processor(
        &self,
        id: SoundProcessorId,
        read_fn: ArrayReadFunc,
    ) -> &[f32] {
        let frame = self.find_processor_state(id);
        read_fn(&frame)
    }

    fn read_local_array_from_sound_processor(
        &self,
        id: SoundProcessorId,
        source_id: SoundExpressionArgumentId,
    ) -> &[f32] {
        self.find_processor_local_array(id, source_id)
    }

    fn get_time_and_speed_at_sound_input(&self, id: SoundInputId) -> (f32, f32) {
        self.time_offset_and_speed_at_input(id)
    }

    fn get_time_and_speed_at_sound_processor(&self, id: SoundProcessorId) -> (f32, f32) {
        self.time_offset_and_speed_at_processor(id)
    }
}

/// An expression context intended for isolated testing and not for
/// use in actual sound processing.
pub(crate) struct MockExpressionContext {
    // array read from by all array read operations
    shared_array: Vec<f32>,
}

impl MockExpressionContext {
    /// Create a new mock expression context
    pub(crate) fn new(len: usize) -> MockExpressionContext {
        let mut shared_array = Vec::new();
        shared_array.resize(len, 0.0);
        MockExpressionContext { shared_array }
    }
}

impl ExpressionContext for MockExpressionContext {
    fn read_scalar_from_sound_input(&self, _id: SoundInputId, _read_fn: ScalarReadFunc) -> f32 {
        0.0
    }

    fn read_scalar_from_sound_processor(
        &self,
        _id: SoundProcessorId,
        _read_fn: ScalarReadFunc,
    ) -> f32 {
        0.0
    }

    fn read_array_from_sound_input(&self, _id: SoundInputId, _read_fn: ArrayReadFunc) -> &[f32] {
        &self.shared_array
    }

    fn read_array_from_sound_processor(
        &self,
        _id: SoundProcessorId,
        _read_fn: ArrayReadFunc,
    ) -> &[f32] {
        &self.shared_array
    }

    fn read_local_array_from_sound_processor(
        &self,
        _id: SoundProcessorId,
        _source_id: SoundExpressionArgumentId,
    ) -> &[f32] {
        &self.shared_array
    }

    fn get_time_and_speed_at_sound_input(&self, _id: SoundInputId) -> (f32, f32) {
        (0.0, 1.0)
    }

    fn get_time_and_speed_at_sound_processor(&self, _id: SoundProcessorId) -> (f32, f32) {
        (0.0, 1.0)
    }
}

/// Convert an ExpressionContext trait object into a pair of usize,
/// intended for crossing a C function call boundary. The trait object
/// can be recovered with usize_pair_to_expression_context.
pub(crate) unsafe fn expression_context_to_usize_pair(
    ctx: &dyn ExpressionContext,
) -> (usize, usize) {
    debug_assert_eq!(
        std::mem::size_of::<&dyn ExpressionContext>(),
        std::mem::size_of::<(usize, usize)>()
    );
    std::mem::transmute(ctx)
}

/// Convert a usize pair returned by expression_context_to_usize_pair
/// back into a pointer to an ExpressionContext trait object. The
/// original trait object must be valid while the pointer is
/// dereferenced.
pub(crate) unsafe fn usize_pair_to_expression_context(
    p: (usize, usize),
) -> *const dyn ExpressionContext {
    debug_assert_eq!(
        std::mem::size_of::<&dyn ExpressionContext>(),
        std::mem::size_of::<(usize, usize)>()
    );
    std::mem::transmute(p)
}
