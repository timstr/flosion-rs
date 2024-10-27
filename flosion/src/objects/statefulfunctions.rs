use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use inkwell::{
    values::{FloatValue, PointerValue},
    FloatPredicate, IntPredicate,
};

use crate::{
    core::{
        expression::{
            expressioninput::ExpressionInput,
            expressionnode::{ExpressionNode, ExpressionNodeVisitor, ExpressionNodeVisitorMut},
        },
        jit::jit::Jit,
        objecttype::{ObjectType, WithObjectType},
        stashing::StashingContext,
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
    input: ExpressionInput,
    speed: ExpressionInput,
}

impl ExpressionNode for LinearApproach {
    fn new(_args: &ParsedArguments) -> LinearApproach {
        LinearApproach {
            input: ExpressionInput::new(0.0),
            speed: ExpressionInput::new(1.0),
        }
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

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input);
        visitor.input(&self.speed);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input);
        visitor.input(&mut self.speed);
    }
}

impl Stashable for LinearApproach {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
        stasher.object(&self.speed);
    }
}

impl UnstashableInplace for LinearApproach {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        unstasher.object_inplace(&mut self.speed)?;
        Ok(())
    }
}

impl WithObjectType for LinearApproach {
    const TYPE: ObjectType = ObjectType::new("linearapproach");
}

// TODO: consider renaming to ExponetialSmooth
pub struct ExponentialApproach {
    input: ExpressionInput,
    decay_rate: ExpressionInput,
}

impl ExpressionNode for ExponentialApproach {
    fn new(_args: &ParsedArguments) -> ExponentialApproach {
        ExponentialApproach {
            input: ExpressionInput::new(0.0),
            decay_rate: ExpressionInput::new(0.5),
        }
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

        // TODO: this struggles numerically with long (but not that long, at audio
        // sample rates) decay times, simply because representing a slow decay from
        // one sample to the next requires multiplying accurately by a fraction
        // a sliver less than 1, and f32 seems to suffer rounding issues here.
        // Consider doing computations and storing state as f64 and converting only
        // the final value to f32

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

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input);
        visitor.input(&self.decay_rate);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input);
        visitor.input(&mut self.decay_rate);
    }
}

impl Stashable for ExponentialApproach {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
        stasher.object(&self.decay_rate);
    }
}

impl UnstashableInplace for ExponentialApproach {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        unstasher.object_inplace(&mut self.decay_rate)?;
        Ok(())
    }
}

impl WithObjectType for ExponentialApproach {
    const TYPE: ObjectType = ObjectType::new("exponentialapproach");
}

pub struct Integrator {
    input: ExpressionInput,
}

impl ExpressionNode for Integrator {
    fn new(_args: &ParsedArguments) -> Integrator {
        Integrator {
            input: ExpressionInput::new(0.0),
        }
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

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input);
    }
}

impl Stashable for Integrator {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
    }
}

impl UnstashableInplace for Integrator {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        Ok(())
    }
}

impl WithObjectType for Integrator {
    const TYPE: ObjectType = ObjectType::new("integrator");
}

pub struct WrappingIntegrator {
    input: ExpressionInput,
}

impl ExpressionNode for WrappingIntegrator {
    fn new(_args: &ParsedArguments) -> WrappingIntegrator {
        WrappingIntegrator {
            input: ExpressionInput::new(0.0),
        }
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

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input);
    }
}

impl Stashable for WrappingIntegrator {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
    }
}

impl UnstashableInplace for WrappingIntegrator {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        Ok(())
    }
}

impl WithObjectType for WrappingIntegrator {
    const TYPE: ObjectType = ObjectType::new("wrappingintegrator");
}
