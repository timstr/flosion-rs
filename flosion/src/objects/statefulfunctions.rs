use inkwell::{
    values::{FloatValue, PointerValue},
    FloatPredicate, IntPredicate,
};

use crate::{
    core::{
        expression::{
            expressionnode::ExpressionNode, expressionnodeinput::ExpressionNodeInputHandle,
            expressionnodetools::ExpressionNodeTools,
        },
        jit::jit::Jit,
        objecttype::{ObjectType, WithObjectType},
    },
    ui_core::arguments::ParsedArguments,
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

impl ExpressionNode for LinearApproach {
    fn new(mut tools: ExpressionNodeTools<'_>, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(LinearApproach {
            _input: tools.add_input(0.0),
            _speed: tools.add_input(1.0),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![jit.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _jit: &mut Jit<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _jit: &mut Jit<'ctx>, _compile_state: &()) {
        ()
    }

    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        _compile_state: &(),
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 2);
        debug_assert_eq!(variables.len(), 1);
        let input = inputs[0];
        let speed = inputs[1];
        let variable = variables[0];

        let step_pos = jit
            .builder()
            .build_float_mul(speed, jit.time_step(), "step_pos")
            .unwrap();
        let step_neg = jit.builder().build_float_neg(step_pos, "step_neg").unwrap();

        let value = jit
            .builder()
            .build_load(jit.types.f32_type, variable, "value")
            .unwrap()
            .into_float_value();
        let value_lt_input = jit
            .builder()
            .build_float_compare(FloatPredicate::OLT, value, input, "value_lt_input")
            .unwrap();
        let step = jit
            .builder()
            .build_select(value_lt_input, step_pos, step_neg, "step")
            .unwrap()
            .into_float_value();
        let value_plus_step = jit
            .builder()
            .build_float_add(value, step, "value_plus_step")
            .unwrap();

        let value_plus_step_lt_input = jit
            .builder()
            .build_float_compare(
                FloatPredicate::OLT,
                value_plus_step,
                input,
                "value_plus_step_lt_input",
            )
            .unwrap();
        let overshoot = jit
            .builder()
            .build_int_compare(
                IntPredicate::NE,
                value_lt_input,
                value_plus_step_lt_input,
                "overshoot",
            )
            .unwrap();
        let new_value = jit
            .builder()
            .build_select(overshoot, input, value_plus_step, "new_value")
            .unwrap()
            .into_float_value();
        jit.builder().build_store(variable, new_value).unwrap();
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

impl ExpressionNode for ExponentialApproach {
    fn new(mut tools: ExpressionNodeTools<'_>, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(ExponentialApproach {
            _input: tools.add_input(0.0),
            _decay_rate: tools.add_input(0.5),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![jit.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _jit: &mut Jit<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _jit: &mut Jit<'ctx>, _compile_state: &()) {
        ()
    }

    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
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
        // other expression node will likely benefit from them
        let ln_a = jit.build_unary_intrinsic_call("llvm.log", decay_rate);
        let b_ln_a = jit
            .builder()
            .build_float_mul(jit.time_step(), ln_a, "b_ln_a")
            .unwrap();
        let decay_amount = jit.build_unary_intrinsic_call("llvm.exp", b_ln_a);
        let one_minus_decay_amount = jit
            .builder()
            .build_float_sub(
                jit.float_type().const_float(1.0),
                decay_amount,
                "one_minus_decay_amt",
            )
            .unwrap();

        let ptr_val = variables[0];
        let prev_val = jit
            .builder()
            .build_load(jit.types.f32_type, ptr_val, "prev_val")
            .unwrap()
            .into_float_value();
        let diff = jit
            .builder()
            .build_float_sub(input, prev_val, "diff")
            .unwrap();
        let scaled_diff = jit
            .builder()
            .build_float_mul(diff, one_minus_decay_amount, "scaled_diff")
            .unwrap();
        let next_val = jit
            .builder()
            .build_float_add(prev_val, scaled_diff, "next_val")
            .unwrap();
        jit.builder().build_store(ptr_val, next_val).unwrap();
        next_val
    }
}

impl WithObjectType for ExponentialApproach {
    const TYPE: ObjectType = ObjectType::new("exponentialapproach");
}

pub struct Integrator {
    _input: ExpressionNodeInputHandle,
}

impl ExpressionNode for Integrator {
    fn new(mut tools: ExpressionNodeTools<'_>, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(Integrator {
            _input: tools.add_input(0.0),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![jit.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _jit: &mut Jit<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _jit: &mut Jit<'ctx>, _compile_state: &()) {}

    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        _compile_state: &(),
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 1);
        let input = inputs[0];
        let input_times_dt = jit
            .builder()
            .build_float_mul(input, jit.time_step(), "input_times_dt")
            .unwrap();
        let variable = variables[0];
        let prev_value = jit
            .builder()
            .build_load(jit.types.f32_type, variable, "prev_value")
            .unwrap()
            .into_float_value();
        let sum = jit
            .builder()
            .build_float_add(input_times_dt, prev_value, "sum")
            .unwrap();
        jit.builder().build_store(variable, sum).unwrap();
        sum
    }
}

impl WithObjectType for Integrator {
    const TYPE: ObjectType = ObjectType::new("integrator");
}

pub struct WrappingIntegrator {
    _input: ExpressionNodeInputHandle,
}

impl ExpressionNode for WrappingIntegrator {
    fn new(mut tools: ExpressionNodeTools<'_>, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(WrappingIntegrator {
            _input: tools.add_input(0.0),
        })
    }

    const NUM_VARIABLES: usize = 1;

    type CompileState<'ctx> = ();

    fn compile_start_over<'ctx>(&self, jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![jit.float_type().const_float(0.0)]
    }

    fn compile_pre_loop<'ctx>(&self, _jit: &mut Jit<'ctx>) -> () {
        ()
    }

    fn compile_post_loop<'ctx>(&self, _jit: &mut Jit<'ctx>, _compile_state: &()) {}

    fn compile_loop<'ctx>(
        &self,
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        _compile_state: &(),
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 1);
        let input = inputs[0];
        let input_times_dt = jit
            .builder()
            .build_float_mul(input, jit.time_step(), "input_times_dt")
            .unwrap();
        let variable = variables[0];
        let prev_value = jit
            .builder()
            .build_load(jit.types.f32_type, variable, "prev_value")
            .unwrap()
            .into_float_value();
        let sum = jit
            .builder()
            .build_float_add(input_times_dt, prev_value, "sum")
            .unwrap();
        let floor_sum = jit.build_unary_intrinsic_call("llvm.floor", sum);
        let fract_sum = jit
            .builder()
            .build_float_sub(sum, floor_sum, "fract_sum")
            .unwrap();
        jit.builder().build_store(variable, fract_sum).unwrap();
        fract_sum
    }
}

impl WithObjectType for WrappingIntegrator {
    const TYPE: ObjectType = ObjectType::new("wrappingintegrator");
}
