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

pub(super) struct WrapperFunctions<'ctx> {
    pub(super) processor_time_wrapper: FunctionValue<'ctx>,
    pub(super) input_time_wrapper: FunctionValue<'ctx>,
    pub(super) argument_pointer_wrapper: FunctionValue<'ctx>,
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

        execution_engine
            .add_global_mapping(&fn_processor_time_wrapper, processor_time_wrapper as usize);
        execution_engine.add_global_mapping(&fn_input_time_wrapper, input_time_wrapper as usize);
        execution_engine
            .add_global_mapping(&fn_argument_pointer, argument_pointer_wrapper as usize);

        WrapperFunctions {
            processor_time_wrapper: fn_processor_time_wrapper,
            input_time_wrapper: fn_input_time_wrapper,
            argument_pointer_wrapper: fn_argument_pointer,
        }
    }
}
