use inkwell::values::{FloatValue, IntValue, PointerValue};

use crate::core::{jit::jit::Jit, sound::argument::ArgumentTranslation};

/// Just an array of floats whose length and discretization
/// are assumed to match what is being evaluated. For example,
/// if you're evaluating an expression over a chunk of samples,
/// the array being pushed must be at least as long as the chunk
/// and its discretization is assumed to be samplewise temporal
/// such that adjacent array indices exactly correspond to
/// adjacent samples.
pub struct PlainF32ArrayArgument;

impl ArgumentTranslation for PlainF32ArrayArgument {
    type PushedType<'a> = &'a [f32];

    type InternalType = (*const f32, usize);

    fn convert_value(slice: &[f32]) -> Self::InternalType {
        (slice.as_ptr(), slice.len())
    }

    fn compile<'ctx>(
        (ptr, len): (PointerValue<'ctx>, IntValue<'ctx>),
        jit: &mut Jit<'ctx>,
    ) -> FloatValue<'ctx> {
        // TODO: check length
        // This will be complicated and will require additional basic blocks spliced
        // into the middle of the loop, and effectively replacing the jit's instruction
        // location pointing to the end of the loop. This will need support from within
        // the jit module itself.
        let ptr_val = unsafe {
            jit.builder()
                .build_gep(
                    jit.types.f32_type,
                    ptr,
                    &[jit.local_variables().loop_counter.into()],
                    "ptr_val",
                )
                .unwrap()
        };

        let value = jit
            .builder()
            .build_load(jit.types.f32_type, ptr_val, "val")
            .unwrap()
            .into_float_value();

        value
    }
}
