use inkwell::values::FloatValue;

use crate::core::{jit::jit::Jit, sound::argument::ArgumentTranslation};

pub struct F32Argument;

impl ArgumentTranslation for F32Argument {
    type PushedType<'a> = f32;

    type InternalType = (f32,);

    fn convert_value(value: f32) -> (f32,) {
        (value,)
    }

    fn compile<'ctx>(
        (value,): (FloatValue<'ctx>,),
        _jit: &mut Jit<'ctx>,
    ) -> inkwell::values::FloatValue<'ctx> {
        value
    }
}
