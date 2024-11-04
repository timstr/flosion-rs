use core::str;

use inkwell::values::FunctionValue;

use crate::core::{
    expression::context::ExpressionContext,
    sound::{
        argument::ProcessorArgumentId,
        soundinput::{ProcessorInputId, SoundInputLocation},
        soundprocessor::SoundProcessorId,
    },
};

use super::types::JitTypes;

pub(super) unsafe extern "C" fn argument_pointer_wrapper(
    ptr_context: *const (),
    argument_id: usize,
) -> *const () {
    let ctx: *const ExpressionContext = ptr_context as _;
    assert!(
        !ctx.is_null(),
        "Attempted to get argument pointer with null context"
    );
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let argid = ProcessorArgumentId::new(argument_id);
    let arg_value = ctx
        .argument_stack()
        .find_argument_ptr(argid)
        .expect("Attempted to find an argument which was not pushed");
    arg_value as _
}

pub(super) unsafe extern "C" fn processor_time_wrapper(
    ptr_context: *const (),
    sound_processor_id: usize,
    ptr_time: *mut f32,
    ptr_speed: *mut f32,
) {
    let ctx: *const ExpressionContext = ptr_context as _;
    assert!(
        !ctx.is_null(),
        "Attempted to get processor time with null context"
    );
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
    assert!(
        !ctx.is_null(),
        "Attempted to get input time with null context"
    );
    let ctx: &ExpressionContext = unsafe { &*ctx };
    let processor_id = SoundProcessorId::new(processor_id);
    let input_id = ProcessorInputId::new(input_id);
    let location = SoundInputLocation::new(processor_id, input_id);
    let (time, speed) = ctx.get_time_and_speed_at_sound_input(location);
    *ptr_time = time;
    *ptr_speed = speed;
}

pub(super) unsafe extern "C" fn print_str_wrapper(ptr_char: *const u8, len: usize) {
    let u8_slice: &[u8] = unsafe { std::slice::from_raw_parts(ptr_char, len) };
    let s = str::from_utf8_unchecked(u8_slice);
    print!("{}", s);
}

pub(super) unsafe extern "C" fn print_usize_dec_wrapper(value: usize) {
    print!("{}", value);
}

pub(super) unsafe extern "C" fn print_usize_hex_wrapper(value: usize) {
    print!("{:#x}", value);
}

pub(super) unsafe extern "C" fn print_f32_wrapper(value: f32) {
    print!("{}", value);
}

pub(super) unsafe extern "C" fn print_ptr_wrapper(value: *const ()) {
    print!("{:#x}", value as usize);
}

pub(super) struct WrapperFunctions<'ctx> {
    pub(super) processor_time_wrapper: FunctionValue<'ctx>,
    pub(super) input_time_wrapper: FunctionValue<'ctx>,
    pub(super) argument_pointer_wrapper: FunctionValue<'ctx>,
    pub(super) print_str_wrapper: FunctionValue<'ctx>,
    pub(super) print_usize_dec_wrapper: FunctionValue<'ctx>,
    pub(super) print_usize_hex_wrapper: FunctionValue<'ctx>,
    pub(super) print_f32_wrapper: FunctionValue<'ctx>,
    pub(super) print_ptr_wrapper: FunctionValue<'ctx>,
}

impl<'ctx> WrapperFunctions<'ctx> {
    pub(super) fn new(
        types: &JitTypes<'ctx>,
        module: &inkwell::module::Module<'ctx>,
        execution_engine: &inkwell::execution_engine::ExecutionEngine<'ctx>,
    ) -> WrapperFunctions<'ctx> {
        let fn_processor_time_wrapper_type = types.void_type.fn_type(
            &[
                // ptr_context
                types.pointer_type.into(),
                // processor_id
                types.usize_type.into(),
                // ptr_time
                types.pointer_type.into(),
                // ptr_speed
                types.pointer_type.into(),
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
                types.pointer_type.into(),
                // ptr_speed
                types.pointer_type.into(),
            ],
            false,
        );

        let fn_print_str_wrapper_type = types.void_type.fn_type(
            &[
                // ptr_char
                types.pointer_type.into(),
                // len
                types.usize_type.into(),
            ],
            false,
        );

        let fn_print_usize_wrapper_type = types.void_type.fn_type(
            &[
                // value
                types.usize_type.into(),
            ],
            false,
        );

        let fn_print_f32_wrapper_type = types.void_type.fn_type(
            &[
                // value
                types.f32_type.into(),
            ],
            false,
        );

        let fn_print_ptr_wrapper_type = types.void_type.fn_type(
            &[
                // value
                types.pointer_type.into(),
            ],
            false,
        );

        let fn_argument_pointer_wrapper_type = types.pointer_type.fn_type(
            &[
                // ptr_context
                types.pointer_type.into(),
                // argument_id
                types.usize_type.into(),
            ],
            false,
        );

        let fn_processor_time_wrapper = module.add_function(
            "processor_time_wrapper",
            fn_processor_time_wrapper_type,
            None,
        );

        let fn_input_time_wrapper =
            module.add_function("input_time_wrapper", fn_input_time_wrapper_type, None);

        let fn_argument_pointer = module.add_function(
            "argument_pointer_wrapper",
            fn_argument_pointer_wrapper_type,
            None,
        );

        let fn_print_str_wrapper =
            module.add_function("print_str_wrapper", fn_print_str_wrapper_type, None);

        let fn_print_usize_dec_wrapper =
            module.add_function("print_usize_dec_wrapper", fn_print_usize_wrapper_type, None);

        let fn_print_usize_hex_wrapper =
            module.add_function("print_usize_hex_wrapper", fn_print_usize_wrapper_type, None);

        let fn_print_f32_wrapper =
            module.add_function("print_f32_wrapper", fn_print_f32_wrapper_type, None);

        let fn_print_ptr_wrapper =
            module.add_function("print_ptr_wrapper", fn_print_ptr_wrapper_type, None);

        execution_engine
            .add_global_mapping(&fn_processor_time_wrapper, processor_time_wrapper as usize);
        execution_engine.add_global_mapping(&fn_input_time_wrapper, input_time_wrapper as usize);
        execution_engine
            .add_global_mapping(&fn_argument_pointer, argument_pointer_wrapper as usize);
        execution_engine.add_global_mapping(&fn_print_str_wrapper, print_str_wrapper as usize);
        execution_engine.add_global_mapping(
            &fn_print_usize_dec_wrapper,
            print_usize_dec_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_print_usize_hex_wrapper,
            print_usize_hex_wrapper as usize,
        );
        execution_engine.add_global_mapping(&fn_print_f32_wrapper, print_f32_wrapper as usize);
        execution_engine.add_global_mapping(&fn_print_ptr_wrapper, print_ptr_wrapper as usize);

        WrapperFunctions {
            processor_time_wrapper: fn_processor_time_wrapper,
            input_time_wrapper: fn_input_time_wrapper,
            argument_pointer_wrapper: fn_argument_pointer,
            print_str_wrapper: fn_print_str_wrapper,
            print_usize_dec_wrapper: fn_print_usize_dec_wrapper,
            print_usize_hex_wrapper: fn_print_usize_hex_wrapper,
            print_f32_wrapper: fn_print_f32_wrapper,
            print_ptr_wrapper: fn_print_ptr_wrapper,
        }
    }
}
