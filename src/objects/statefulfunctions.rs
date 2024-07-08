use inkwell::{
    values::{FloatValue, PointerValue},
    FloatPredicate, IntPredicate,
};

use crate::core::{
    expression::{
        expressionnode::StatefulExpressionNode, expressionnodeinput::ExpressionNodeInputHandle,
        expressionnodetools::ExpressionNodeTools,
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::codegen::CodeGen,
};

// TODO: min
// TODO: max
// TODO: prev
// TODO: random (LFSR?)
// TODO: flip flop

// TODO: consider renaming to LinearSmooth
pub struct LinearApproach {
    _input: ExpressionNodeInputHandle,
    _speed: ExpressionNodeInputHandle,
}

impl StatefulExpressionNode for LinearApproach {
    fn new(mut tools: ExpressionNodeTools<'_>, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(LinearApproach {
            _input: tools.add_input(0.0),
            _speed: tools.add_input(1.0),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
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
        let speed = inputs[1];
        let variable = variables[0];

        let step_pos = codegen
            .builder()
            .build_float_mul(speed, codegen.time_step(), "step_pos")
            .unwrap();
        let step_neg = codegen
            .builder()
            .build_float_neg(step_pos, "step_neg")
            .unwrap();

        let value = codegen
            .builder()
            .build_load(variable, "value")
            .unwrap()
            .into_float_value();
        let value_lt_input = codegen
            .builder()
            .build_float_compare(FloatPredicate::OLT, value, input, "value_lt_input")
            .unwrap();
        let step = codegen
            .builder()
            .build_select(value_lt_input, step_pos, step_neg, "step")
            .unwrap()
            .into_float_value();
        let value_plus_step = codegen
            .builder()
            .build_float_add(value, step, "value_plus_step")
            .unwrap();

        let value_plus_step_lt_input = codegen
            .builder()
            .build_float_compare(
                FloatPredicate::OLT,
                value_plus_step,
                input,
                "value_plus_step_lt_input",
            )
            .unwrap();
        let overshoot = codegen
            .builder()
            .build_int_compare(
                IntPredicate::NE,
                value_lt_input,
                value_plus_step_lt_input,
                "overshoot",
            )
            .unwrap();
        let new_value = codegen
            .builder()
            .build_select(overshoot, input, value_plus_step, "new_value")
            .unwrap()
            .into_float_value();
        codegen.builder().build_store(variable, new_value).unwrap();
        new_value
    }
}

impl WithObjectType for LinearApproach {
    const TYPE: ObjectType = ObjectType::new("linearapproach");
}

// TODO: consider renaming to ExponetialSmooth
pub struct ExponentialApproach {
    _input: ExpressionNodeInputHandle,
    _decay_rate: ExpressionNodeInputHandle,
}

impl StatefulExpressionNode for ExponentialApproach {
    fn new(mut tools: ExpressionNodeTools<'_>, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(ExponentialApproach {
            _input: tools.add_input(0.0),
            _decay_rate: tools.add_input(0.5),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
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
            .build_float_mul(codegen.time_step(), ln_a, "b_ln_a")
            .unwrap();
        let decay_amount = codegen.build_unary_intrinsic_call("llvm.exp", b_ln_a);
        let one_minus_decay_amount = codegen
            .builder()
            .build_float_sub(
                codegen.float_type().const_float(1.0),
                decay_amount,
                "one_minus_decay_amt",
            )
            .unwrap();

        let ptr_val = variables[0];
        let prev_val = codegen
            .builder()
            .build_load(ptr_val, "prev_val")
            .unwrap()
            .into_float_value();
        let diff = codegen
            .builder()
            .build_float_sub(input, prev_val, "diff")
            .unwrap();
        let scaled_diff = codegen
            .builder()
            .build_float_mul(diff, one_minus_decay_amount, "scaled_diff")
            .unwrap();
        let next_val = codegen
            .builder()
            .build_float_add(prev_val, scaled_diff, "next_val")
            .unwrap();
        codegen.builder().build_store(ptr_val, next_val).unwrap();
        next_val
    }
}

impl WithObjectType for ExponentialApproach {
    const TYPE: ObjectType = ObjectType::new("exponentialapproach");
}

pub struct Integrator {
    _input: ExpressionNodeInputHandle,
}

impl StatefulExpressionNode for Integrator {
    fn new(mut tools: ExpressionNodeTools<'_>, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Integrator {
            _input: tools.add_input(0.0),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![codegen.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _codegen: &mut CodeGen<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _codegen: &mut CodeGen<'ctx>, _compile_state: &()) {}

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        _compile_state: &(),
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 1);
        let input = inputs[0];
        let input_times_dt = codegen
            .builder()
            .build_float_mul(input, codegen.time_step(), "input_times_dt")
            .unwrap();
        let variable = variables[0];
        let prev_value = codegen
            .builder()
            .build_load(variable, "prev_value")
            .unwrap()
            .into_float_value();
        let sum = codegen
            .builder()
            .build_float_add(input_times_dt, prev_value, "sum")
            .unwrap();
        codegen.builder().build_store(variable, sum).unwrap();
        sum
    }
}

impl WithObjectType for Integrator {
    const TYPE: ObjectType = ObjectType::new("integrator");
}

pub struct WrappingIntegrator {
    _input: ExpressionNodeInputHandle,
}

impl StatefulExpressionNode for WrappingIntegrator {
    fn new(mut tools: ExpressionNodeTools<'_>, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WrappingIntegrator {
            _input: tools.add_input(0.0),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![codegen.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _codegen: &mut CodeGen<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _codegen: &mut CodeGen<'ctx>, _compile_state: &()) {}

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        _compile_state: &(),
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 1);
        let input = inputs[0];
        let input_times_dt = codegen
            .builder()
            .build_float_mul(input, codegen.time_step(), "input_times_dt")
            .unwrap();
        let variable = variables[0];
        let prev_value = codegen
            .builder()
            .build_load(variable, "prev_value")
            .unwrap()
            .into_float_value();
        let sum = codegen
            .builder()
            .build_float_add(input_times_dt, prev_value, "sum")
            .unwrap();
        let floor_sum = codegen.build_unary_intrinsic_call("llvm.floor", sum);
        let fract_sum = codegen
            .builder()
            .build_float_sub(sum, floor_sum, "fract_sum")
            .unwrap();
        codegen.builder().build_store(variable, fract_sum).unwrap();
        fract_sum
    }
}

impl WithObjectType for WrappingIntegrator {
    const TYPE: ObjectType = ObjectType::new("wrappingintegrator");
}
