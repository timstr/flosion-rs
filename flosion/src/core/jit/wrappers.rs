use std::any::Any;

use inkwell::values::FunctionValue;

use crate::core::{
    expression::context::ExpressionContext,
    sound::{
        expressionargument::ProcessorArgumentId,
        soundinput::{ProcessorInputId, SoundInputLocation},
        soundprocessor::SoundProcessorId,
    },
};

use super::types::JitTypes;

pub type ScalarReadFunc = fn(&dyn Any) -> f32;
pub type ArrayReadFunc = for<'a> fn(&'a dyn Any) -> &'a [f32];

pub(super) unsafe extern "C" fn input_scalar_read_wrapper(
    scalar_read_fn: *const (),
    ptr_context: *const (),
    processor_id: usize,
    input_id: usize,
) -> f32 {
    assert_eq!(
        std::mem::size_of::<ScalarReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let read_fn: ScalarReadFunc = std::mem::transmute(scalar_read_fn);
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let processor_id = SoundProcessorId::new(processor_id);
    let input_id = ProcessorInputId::new(input_id);
    let location = SoundInputLocation::new(processor_id, input_id);
    ctx.read_scalar_from_input_state(location, read_fn)
}

pub(super) unsafe extern "C" fn processor_scalar_read_wrapper(
    scalar_read_fn: *const (),
    ptr_context: *const (),
    sound_processor_id: usize,
) -> f32 {
    assert_eq!(
        std::mem::size_of::<ScalarReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let read_fn: ScalarReadFunc = std::mem::transmute(scalar_read_fn);
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    ctx.read_scalar_from_processor_state(spid, read_fn)
}

pub(super) unsafe extern "C" fn input_array_read_wrapper(
    array_read_fn: *const (),
    ptr_context: *const (),
    processor_id: usize,
    input_id: usize,
    expected_len: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let processor_id = SoundProcessorId::new(processor_id);
    let input_id = ProcessorInputId::new(input_id);
    let location = SoundInputLocation::new(processor_id, input_id);
    let s = ctx.read_array_from_input_state(location, f);
    if s.len() != expected_len {
        panic!("input_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

pub(super) unsafe extern "C" fn processor_array_read_wrapper(
    array_read_fn: *const (),
    ptr_context: *const (),
    sound_processor_id: usize,
    expected_len: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    let s = ctx.read_array_from_processor_state(spid, f);
    if s.len() != expected_len {
        panic!("processor_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

pub(super) unsafe extern "C" fn processor_time_wrapper(
    ptr_context: *const (),
    sound_processor_id: usize,
    ptr_time: *mut f32,
    ptr_speed: *mut f32,
) {
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    let (time, speed) = ctx.get_time_and_speed_at_sound_processor(spid);
    *ptr_time = time;
    *ptr_speed = speed;
}

pub(super) unsafe extern "C" fn input_time_wrapper(
    ptr_context: *const (),
    processor_id: usize,
    input_id: usize,
    ptr_time: *mut f32,
    ptr_speed: *mut f32,
) {
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let processor_id = SoundProcessorId::new(processor_id);
    let input_id = ProcessorInputId::new(input_id);
    let location = SoundInputLocation::new(processor_id, input_id);
    let (time, speed) = ctx.get_time_and_speed_at_sound_input(location);
    *ptr_time = time;
    *ptr_speed = speed;
}

pub(super) unsafe extern "C" fn processor_local_array_read_wrapper(
    ptr_context: *const (),
    sound_processor_id: usize,
    argument_id: usize,
    expected_len: usize,
) -> *const f32 {
    let ctx: *const ExpressionContext = ptr_context as _;
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let spid = SoundProcessorId::new(sound_processor_id);
    let arg_id = ProcessorArgumentId::new(argument_id);
    let s = ctx.read_local_array_from_sound_processor(spid, arg_id);
    if s.len() != expected_len {
        panic!("processor_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

pub(super) struct WrapperFunctions<'ctx> {
    pub(super) processor_scalar_read_wrapper: FunctionValue<'ctx>,
    pub(super) input_scalar_read_wrapper: FunctionValue<'ctx>,
    pub(super) processor_array_read_wrapper: FunctionValue<'ctx>,
    pub(super) input_array_read_wrapper: FunctionValue<'ctx>,
    pub(super) processor_local_array_read_wrapper: FunctionValue<'ctx>,
    pub(super) processor_time_wrapper: FunctionValue<'ctx>,
    pub(super) input_time_wrapper: FunctionValue<'ctx>,
}

impl<'ctx> WrapperFunctions<'ctx> {
    pub(super) fn new(
        types: &JitTypes<'ctx>,
        module: &inkwell::module::Module<'ctx>,
        execution_engine: &inkwell::execution_engine::ExecutionEngine<'ctx>,
    ) -> WrapperFunctions<'ctx> {
        let fn_processor_scalar_read_wrapper_type = types.f32_type.fn_type(
            &[
                // array_read_fn
                types.usize_type.into(),
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
            ],
            false,
        );

        let fn_input_scalar_read_wrapper_type = types.f32_type.fn_type(
            &[
                // array_read_fn
                types.usize_type.into(),
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
                // input_id
                types.usize_type.into(),
            ],
            false,
        );

        let fn_processor_array_read_wrapper_type = types.f32_pointer_type.fn_type(
            &[
                // array_read_fn
                types.pointer_type.into(),
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
                // expected_len
                types.usize_type.into(),
            ],
            false,
        );

        let fn_input_array_read_wrapper_type = types.f32_pointer_type.fn_type(
            &[
                // array_read_fn
                types.pointer_type.into(),
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
                // input_id
                types.usize_type.into(),
                // expected_len
                types.usize_type.into(),
            ],
            false,
        );

        let fn_processor_time_wrapper_type = types.void_type.fn_type(
            &[
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
                // ptr_time
                types.f32_pointer_type.into(),
                // ptr_speed
                types.f32_pointer_type.into(),
            ],
            false,
        );

        let fn_input_time_wrapper_type = types.void_type.fn_type(
            &[
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
                // input_id
                types.usize_type.into(),
                // ptr_time
                types.f32_pointer_type.into(),
                // ptr_speed
                types.f32_pointer_type.into(),
            ],
            false,
        );

        let fn_local_array_read_wrapper = types.f32_pointer_type.fn_type(
            &[
                // ptr_context
                types.pointer_type.into(),
                // sound_processor_id
                types.usize_type.into(),
                // argument_id
                types.usize_type.into(),
                // expected_len
                types.usize_type.into(),
            ],
            false,
        );

        let fn_input_scalar_read_wrapper = module.add_function(
            "input_scalar_read_wrapper",
            fn_input_scalar_read_wrapper_type,
            None,
        );

        let fn_proc_scalar_read_wrapper = module.add_function(
            "proc_scalar_read_wrapper",
            fn_processor_scalar_read_wrapper_type,
            None,
        );

        let fn_proc_array_read_wrapper = module.add_function(
            "processor_array_read_wrapper",
            fn_processor_array_read_wrapper_type,
            None,
        );

        let fn_input_array_read_wrapper = module.add_function(
            "input_array_read_wrapper",
            fn_input_array_read_wrapper_type,
            None,
        );

        let fn_processor_time_wrapper = module.add_function(
            "processor_time_wrapper",
            fn_processor_time_wrapper_type,
            None,
        );

        let fn_input_time_wrapper =
            module.add_function("input_time_wrapper", fn_input_time_wrapper_type, None);

        let fn_proc_local_array_read_wrapper = module.add_function(
            "processor_local_array_wrapper",
            fn_local_array_read_wrapper,
            None,
        );

        execution_engine.add_global_mapping(
            &fn_input_scalar_read_wrapper,
            input_scalar_read_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_proc_scalar_read_wrapper,
            processor_scalar_read_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_proc_array_read_wrapper,
            processor_array_read_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_input_array_read_wrapper,
            input_array_read_wrapper as usize,
        );
        execution_engine
            .add_global_mapping(&fn_processor_time_wrapper, processor_time_wrapper as usize);
        execution_engine.add_global_mapping(&fn_input_time_wrapper, input_time_wrapper as usize);
        execution_engine.add_global_mapping(
            &fn_proc_local_array_read_wrapper,
            processor_local_array_read_wrapper as usize,
        );

        WrapperFunctions {
            processor_scalar_read_wrapper: fn_proc_scalar_read_wrapper,
            input_scalar_read_wrapper: fn_input_scalar_read_wrapper,
            processor_array_read_wrapper: fn_proc_array_read_wrapper,
            input_array_read_wrapper: fn_input_array_read_wrapper,
            processor_local_array_read_wrapper: fn_proc_local_array_read_wrapper,
            processor_time_wrapper: fn_processor_time_wrapper,
            input_time_wrapper: fn_input_time_wrapper,
        }
    }
}