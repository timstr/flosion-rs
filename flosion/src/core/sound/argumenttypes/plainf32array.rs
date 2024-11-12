use inkwell::{
    values::{FloatValue, IntValue, PointerValue},
    IntPredicate,
};

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
        let bb_in_bounds = jit
            .context()
            .append_basic_block(jit.function(), "plainf32array_in_bounds");

        let bb_out_of_bounds = jit
            .context()
            .append_basic_block(jit.function(), "plainf32array_in_bounds");

        let bb_loop_body_rest = jit
            .context()
            .append_basic_block(jit.function(), "plainf32array_loop_body_rest");

        let ptr_local_val = jit
            .builder()
            .build_alloca(jit.types.f32_type, "ptr_local_val")
            .unwrap();

        let idx_in_bounds = jit
            .builder()
            .build_int_compare(
                IntPredicate::ULT,
                jit.local_variables().loop_counter,
                len,
                "idx_in_bounds",
            )
            .unwrap();

        jit.builder()
            .build_conditional_branch(idx_in_bounds, bb_in_bounds, bb_out_of_bounds)
            .unwrap();

        jit.builder().position_at_end(bb_in_bounds);
        {
            let ptr_array_val = unsafe {
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
                .build_load(jit.types.f32_type, ptr_array_val, "val")
                .unwrap()
                .into_float_value();

            jit.builder().build_store(ptr_local_val, value).unwrap();

            jit.builder()
                .build_unconditional_branch(bb_loop_body_rest)
                .unwrap();
        }

        jit.builder().position_at_end(bb_out_of_bounds);
        {
            // Padding with zero
            let pad_value = jit.types.f32_type.const_float(0.0);

            jit.builder().build_store(ptr_local_val, pad_value).unwrap();

            jit.builder()
                .build_unconditional_branch(bb_loop_body_rest)
                .unwrap();
        }

        jit.builder().position_at_end(bb_loop_body_rest);
        jit.replace_loop_body(bb_loop_body_rest);

        let value = jit
            .builder()
            .build_load(jit.types.f32_type, ptr_local_val, "val")
            .unwrap()
            .into_float_value();

        value
    }
}
