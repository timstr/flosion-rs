use inkwell::values::{FloatValue, PointerValue};

use crate::core::{
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::codegen::CodeGen,
    number::{
        numberinput::NumberInputHandle, numbersource::StatefulNumberSource,
        numbersourcetools::NumberSourceTools,
    },
};

pub struct ExponentialApproach {
    _input: NumberInputHandle,
    _decay_rate: NumberInputHandle,
}

impl StatefulNumberSource for ExponentialApproach {
    fn new(mut tools: NumberSourceTools<'_>, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(ExponentialApproach {
            _input: tools.add_number_input(0.0),
            _decay_rate: tools.add_number_input(0.5),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_init<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![codegen.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _codegen: &mut CodeGen<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _codegen: &mut CodeGen<'ctx>, _compile_state: &()) {
        ()
    }

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        _compile_state: &(),
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 2);
        debug_assert_eq!(variables.len(), 1);
        let input = inputs[0];
        let decay_rate = inputs[1];

        // Copied from Pow
        // TODO: put this and similar helpers somewhere shared, many
        // other number sources will likely benefit from them
        let ln_a = codegen.build_unary_intrinsic_call("llvm.log", decay_rate);
        let b_ln_a = codegen
            .builder()
            .build_float_mul(codegen.time_step(), ln_a, "b_ln_a");
        let decay_amount = codegen.build_unary_intrinsic_call("llvm.exp", b_ln_a);
        let one_minus_decay_amount = codegen.builder().build_float_sub(
            codegen.float_type().const_float(1.0),
            decay_amount,
            "one_minus_decay_amt",
        );

        let ptr_val = variables[0];
        let prev_val = codegen
            .builder()
            .build_load(ptr_val, "prev_val")
            .into_float_value();
        let diff = codegen.builder().build_float_sub(input, prev_val, "diff");
        let scaled_diff =
            codegen
                .builder()
                .build_float_mul(diff, one_minus_decay_amount, "scaled_diff");
        let next_val = codegen
            .builder()
            .build_float_add(prev_val, scaled_diff, "next_val");
        codegen.builder().build_store(ptr_val, next_val);
        next_val
    }
}

impl WithObjectType for ExponentialApproach {
    const TYPE: ObjectType = ObjectType::new("exponentialapproach");
}
