use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    sound::{
        context::Context, soundinput::SoundInputId, expressionargument::SoundExpressionArgumentId,
        soundprocessor::SoundProcessorId,
    },
};

pub(crate) trait ExpressionContext {
    fn read_scalar_from_sound_input(&self, id: SoundInputId, read_fn: ScalarReadFunc) -> f32;

    fn read_scalar_from_sound_processor(
        &self,
        id: SoundProcessorId,
        read_fn: ScalarReadFunc,
    ) -> f32;

    fn read_array_from_sound_input(&self, id: SoundInputId, read_fn: ArrayReadFunc) -> &[f32];

    fn read_array_from_sound_processor(
        &self,
        id: SoundProcessorId,
        read_fn: ArrayReadFunc,
    ) -> &[f32];

    fn read_local_array_from_sound_processor(
        &self,
        id: SoundProcessorId,
        source_id: SoundExpressionArgumentId,
    ) -> &[f32];

    fn get_time_and_speed_at_sound_input(&self, id: SoundInputId) -> (f32, f32);

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

pub(crate) struct MockExpressionContext {
    // array read from by all array read operations
    shared_array: Vec<f32>,
}

impl MockExpressionContext {
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

pub(crate) unsafe fn expression_context_to_usize_pair(
    ctx: &dyn ExpressionContext,
) -> (usize, usize) {
    debug_assert_eq!(
        std::mem::size_of::<&dyn ExpressionContext>(),
        std::mem::size_of::<(usize, usize)>()
    );
    std::mem::transmute(ctx)
}

pub(crate) unsafe fn usize_pair_to_expression_context(
    p: (usize, usize),
) -> *const dyn ExpressionContext {
    debug_assert_eq!(
        std::mem::size_of::<&dyn ExpressionContext>(),
        std::mem::size_of::<(usize, usize)>()
    );
    std::mem::transmute(p)
}
