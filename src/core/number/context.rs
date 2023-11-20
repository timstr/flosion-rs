use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    sound::{context::Context, soundinput::SoundInputId, soundprocessor::SoundProcessorId},
};

pub(crate) trait NumberContext {
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

    fn get_time_and_speed_at_sound_input(&self, id: SoundInputId) -> (f32, f32);

    fn get_time_and_speed_at_sound_processor(&self, id: SoundProcessorId) -> (f32, f32);
}

impl<'ctx> NumberContext for Context<'ctx> {
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

    fn get_time_and_speed_at_sound_input(&self, id: SoundInputId) -> (f32, f32) {
        self.time_offset_and_speed_at_input(id)
    }

    fn get_time_and_speed_at_sound_processor(&self, id: SoundProcessorId) -> (f32, f32) {
        self.time_offset_and_speed_at_processor(id)
    }
}

pub(crate) struct MockNumberContext {
    // array read from by all array read operations
    shared_array: Vec<f32>,
}

impl MockNumberContext {
    pub(crate) fn new(len: usize) -> MockNumberContext {
        let mut shared_array = Vec::new();
        shared_array.resize(len, 0.0);
        MockNumberContext { shared_array }
    }
}

impl NumberContext for MockNumberContext {
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

    fn get_time_and_speed_at_sound_input(&self, _id: SoundInputId) -> (f32, f32) {
        (0.0, 1.0)
    }

    fn get_time_and_speed_at_sound_processor(&self, _id: SoundProcessorId) -> (f32, f32) {
        (0.0, 1.0)
    }
}

pub(crate) unsafe fn number_context_to_usize_pair(ctx: &dyn NumberContext) -> (usize, usize) {
    debug_assert_eq!(
        std::mem::size_of::<&dyn NumberContext>(),
        std::mem::size_of::<(usize, usize)>()
    );
    std::mem::transmute(ctx)
}

pub(crate) unsafe fn usize_pair_to_number_context(p: (usize, usize)) -> *const dyn NumberContext {
    debug_assert_eq!(
        std::mem::size_of::<&dyn NumberContext>(),
        std::mem::size_of::<(usize, usize)>()
    );
    std::mem::transmute(p)
}
