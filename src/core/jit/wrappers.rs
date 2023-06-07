use crate::core::{
    anydata::AnyData,
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{context::Context, soundinput::SoundInputId, soundprocessor::SoundProcessorId},
};

pub type ScalarReadFunc = fn(&AnyData) -> f32;
pub type ArrayReadFunc = for<'a> fn(&'a AnyData<'a>) -> &'a [f32];

pub(super) unsafe extern "C" fn input_scalar_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_input_id: usize,
) -> f32 {
    assert_eq!(
        std::mem::size_of::<ScalarReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ScalarReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let siid = SoundInputId::new(sound_input_id);
    let frame = ctx.find_input_frame(siid);
    f(&frame.state())
}

pub(super) unsafe extern "C" fn processor_scalar_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_processor_id: usize,
) -> f32 {
    assert_eq!(
        std::mem::size_of::<ScalarReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ScalarReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    let frame = ctx.find_processor_state(spid);
    f(&frame)
}

pub(super) unsafe extern "C" fn input_array_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_input_id: usize,
    expected_len: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let siid = SoundInputId::new(sound_input_id);
    let frame = ctx.find_input_frame(siid);
    let s = f(&frame.state());
    if s.len() != expected_len {
        panic!("input_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

pub(super) unsafe extern "C" fn processor_array_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_processor_id: usize,
    expected_len: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    let frame = ctx.find_processor_state(spid);
    let s = f(&frame);
    if s.len() != expected_len {
        panic!("processor_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

pub(super) unsafe extern "C" fn processor_time_wrapper(
    context_ptr: *const (),
    sound_processor_id: usize,
    ptr_time: *mut f32,
    ptr_speed: *mut f32,
) {
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    let (time, speed) = ctx.time_offset_and_speed_at_processor(spid);
    *ptr_time = time;
    *ptr_speed = speed / SAMPLE_FREQUENCY as f32;
}

pub(super) unsafe extern "C" fn input_time_wrapper(
    context_ptr: *const (),
    sound_input_id: usize,
    ptr_time: *mut f32,
    ptr_speed: *mut f32,
) {
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let siid = SoundInputId::new(sound_input_id);
    let (time, speed) = ctx.time_offset_and_speed_at_input(siid);
    *ptr_time = time;
    *ptr_speed = speed / SAMPLE_FREQUENCY as f32;
}
